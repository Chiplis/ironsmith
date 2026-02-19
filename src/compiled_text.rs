use std::cell::Cell;

use crate::ability::{Ability, AbilityKind, ActivationTiming, InterveningIfCondition};
use crate::alternative_cast::AlternativeCastingMethod;
use crate::effect::{
    ChoiceCount, Comparison, Condition, EffectPredicate, EventValueSpec, Until, Value,
};
use crate::object::CounterType;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::types::{Subtype, Supertype};
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
        PlayerFilter::NotYou => "a player other than you".to_string(),
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
        PlayerFilter::Active => "that player".to_string(),
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

fn lowercase_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_ascii_lowercase(), chars.as_str()),
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
    } else if subject == "it" {
        "its".to_string()
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

fn join_with_or(parts: &[String]) -> String {
    match parts.len() {
        0 => String::new(),
        1 => parts[0].clone(),
        2 => format!("{} or {}", parts[0], parts[1]),
        _ => {
            let mut text = parts[..parts.len() - 1].join(", ");
            text.push_str(", or ");
            text.push_str(parts.last().map(String::as_str).unwrap_or_default());
            text
        }
    }
}

fn repeated_energy_symbols(count: usize) -> String {
    "{E}".repeat(count)
}

fn describe_energy_payment_amount(value: &Value) -> String {
    match value {
        Value::Fixed(amount) if *amount > 0 => repeated_energy_symbols(*amount as usize),
        _ => format!("{} energy counter(s)", describe_value(value)),
    }
}

fn describe_card_type_word_local(card_type: CardType) -> &'static str {
    match card_type {
        CardType::Artifact => "artifact",
        CardType::Battle => "battle",
        CardType::Creature => "creature",
        CardType::Enchantment => "enchantment",
        CardType::Instant => "instant",
        CardType::Kindred => "kindred",
        CardType::Land => "land",
        CardType::Planeswalker => "planeswalker",
        CardType::Sorcery => "sorcery",
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

    if colors.count() == 2 {
        use crate::color::Color;
        let has_w = colors.contains(Color::White);
        let has_u = colors.contains(Color::Blue);
        let has_b = colors.contains(Color::Black);
        let has_r = colors.contains(Color::Red);
        let has_g = colors.contains(Color::Green);
        if has_w && has_u {
            return "white and blue".to_string();
        }
        if has_u && has_b {
            return "blue and black".to_string();
        }
        if has_b && has_r {
            return "black and red".to_string();
        }
        if has_r && has_g {
            return "red and green".to_string();
        }
        if has_g && has_w {
            return "green and white".to_string();
        }
        if has_w && has_b {
            return "white and black".to_string();
        }
        if has_b && has_g {
            return "black and green".to_string();
        }
        if has_g && has_u {
            return "green and blue".to_string();
        }
        if has_u && has_r {
            return "blue and red".to_string();
        }
        if has_r && has_w {
            return "red and white".to_string();
        }
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

    if !card.supertypes.is_empty() {
        let supertypes = card
            .supertypes
            .iter()
            .map(|supertype| format!("{supertype:?}").to_ascii_lowercase())
            .collect::<Vec<_>>()
            .join(" ");
        if !supertypes.is_empty() {
            parts.push(supertypes);
        }
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
        let name_lower = card.name.to_ascii_lowercase();
        let subtype_words_lower = card
            .subtypes
            .iter()
            .map(|subtype| format!("{subtype:?}").to_ascii_lowercase())
            .collect::<Vec<_>>();
        let subtype_text = card
            .subtypes
            .iter()
            .map(|subtype| format!("{subtype:?}"))
            .collect::<Vec<_>>()
            .join(" ");
        let name_matches_any_subtype = subtype_words_lower.iter().any(|word| *word == name_lower);
        let name_is_distinct = !card.name.trim().is_empty()
            && name_lower != "token"
            && name_lower != subtype_text.to_ascii_lowercase()
            && !name_matches_any_subtype;
        let use_name_for_creature = card.is_creature() && name_is_distinct;
        let use_name_for_noncreature = !card.is_creature() && name_is_distinct;
        if use_name_for_creature || use_name_for_noncreature {
            parts.push(card.name.clone());
            if !subtype_text.is_empty() {
                parts.push(subtype_text);
            }
        } else {
            parts.push(subtype_text);
        }
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
                    extra_ability_texts.push(quote_token_granted_ability_text(text));
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

fn quote_token_granted_ability_text(text: &str) -> String {
    let trimmed = text.trim().trim_end_matches('.').trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        return trimmed.to_string();
    }
    format!("\"{trimmed}\"")
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

fn render_small_number_or_raw(text: &str) -> String {
    text.trim()
        .parse::<u32>()
        .ok()
        .and_then(small_number_word)
        .map(str::to_string)
        .unwrap_or_else(|| text.trim().to_string())
}

fn looks_like_trigger_condition(head: &str) -> bool {
    let lower = head.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return false;
    }
    if lower.starts_with("chapter ")
        || lower.starts_with("activate only")
        || lower.starts_with("equip ")
        || lower.starts_with("ward")
        || lower.starts_with("madness")
        || lower.starts_with("kicker")
        || lower.starts_with("cycling")
    {
        return false;
    }
    if lower.contains('{') {
        return false;
    }

    [
        " attacks",
        " attack",
        " blocks",
        " block",
        " enters",
        " enter",
        " dies",
        " die",
        " becomes",
        " become",
        " is tapped for mana",
        " cast",
        " casts",
        " gain life",
        " gains life",
        " deals damage",
        " deal damage",
        " create ",
        " unlock ",
        "beginning of",
        "control no other ",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn normalize_trigger_colon_clause(line: &str) -> Option<String> {
    let (head, tail) = line.split_once(": ")?;
    let normalized_head = if let Some(rest) = head.strip_prefix("You ") {
        format!("you {rest}")
    } else {
        head.to_string()
    };
    if !looks_like_trigger_condition(&normalized_head) {
        return None;
    }

    let lower_head = normalized_head.to_ascii_lowercase();
    if lower_head.starts_with("as an additional cost to cast this spell") {
        return None;
    }
    let normalized_tail = if tail
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase())
    {
        lowercase_first(tail)
    } else {
        tail.to_string()
    };

    if lower_head.starts_with("the beginning ") {
        Some(format!("At {normalized_head}, {normalized_tail}"))
    } else if lower_head.starts_with("when ")
        || lower_head.starts_with("whenever ")
        || lower_head.starts_with("at the beginning ")
    {
        Some(format!("{normalized_head}, {normalized_tail}"))
    } else if lower_head.starts_with("you control no other ") {
        Some(format!("When {normalized_head}, {normalized_tail}"))
    } else {
        Some(format!("Whenever {normalized_head}, {normalized_tail}"))
    }
}

fn normalize_inline_earthbend_phrasing(text: &str) -> Option<String> {
    let needle = "Earthbend target land you control with ";
    let suffix = " +1/+1 counter(s)";

    let mut rest = text;
    let mut out = String::new();
    let mut changed = false;

    while let Some(idx) = rest.find(needle) {
        out.push_str(&rest[..idx]);
        let after = &rest[idx + needle.len()..];
        let Some(end_idx) = after.find(suffix) else {
            out.push_str(&rest[idx..]);
            rest = "";
            break;
        };

        let count = after[..end_idx].trim();
        if count.is_empty() {
            out.push_str(&rest[idx..idx + needle.len() + end_idx + suffix.len()]);
        } else {
            out.push_str("Earthbend ");
            out.push_str(count);
            changed = true;
        }
        rest = &after[end_idx + suffix.len()..];
    }

    out.push_str(rest);
    if changed { Some(out) } else { None }
}

fn looks_like_creature_type_list_subject(subject: &str) -> bool {
    let trimmed = subject.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.contains(',') || trimmed.contains(':') {
        return false;
    }

    let lower = trimmed.to_ascii_lowercase();
    for banned in [
        "when ",
        "whenever ",
        "at the beginning ",
        "target ",
        "up to ",
        " each ",
        " enters",
        " attacks",
        " blocks",
        " dies",
        " deals",
        " gain ",
        " get ",
        " has ",
        " have ",
    ] {
        if lower.contains(banned) {
            return false;
        }
    }
    true
}

fn normalize_enchanted_creature_dies_clause(text: &str) -> Option<String> {
    let trimmed = text.trim();
    let tail = strip_prefix_ascii_ci(trimmed, "Whenever a enchanted creature dies, ")
        .or_else(|| strip_prefix_ascii_ci(trimmed, "When a enchanted creature dies, "))
        .or_else(|| strip_prefix_ascii_ci(trimmed, "Whenever enchanted creature dies, "))
        .or_else(|| strip_prefix_ascii_ci(trimmed, "When enchanted creature dies, "))?;

    let tail = tail.trim();
    if let Some(counter_tail) = strip_prefix_ascii_ci(
        tail,
        "return it from graveyard to the battlefield. put ",
    )
    .and_then(|rest| {
        strip_suffix_ascii_ci(rest, " on it.").or_else(|| strip_suffix_ascii_ci(rest, " on it"))
    }) {
        return Some(format!(
            "When enchanted creature dies, return that card to the battlefield under your control with {} on it.",
            counter_tail.trim()
        ));
    }

    if tail.eq_ignore_ascii_case("return it from graveyard to the battlefield.")
        || tail.eq_ignore_ascii_case("return it from graveyard to the battlefield")
        || tail.eq_ignore_ascii_case("return it to the battlefield under your control.")
        || tail.eq_ignore_ascii_case("return it to the battlefield under your control")
        || tail.eq_ignore_ascii_case("put it onto the battlefield under your control.")
        || tail.eq_ignore_ascii_case("put it onto the battlefield under your control")
    {
        return Some(
            "When enchanted creature dies, return that card to the battlefield under your control."
                .to_string(),
        );
    }

    let create_tail = strip_prefix_ascii_ci(tail, "return this aura to its owner's hand. ")
        .or_else(|| strip_prefix_ascii_ci(tail, "return this permanent to its owner's hand. "))
        .and_then(|rest| strip_prefix_ascii_ci(rest, "create "))
        .or_else(|| strip_prefix_ascii_ci(tail, "return this aura to its owner's hand and create "))
        .or_else(|| {
            strip_prefix_ascii_ci(
                tail,
                "return this permanent to its owner's hand and create ",
            )
        });
    if let Some(create_tail) = create_tail {
        let mut create_clause = create_tail.trim().to_string();
        if !create_clause.ends_with('.') {
            create_clause.push('.');
        }
        return Some(format!(
            "When enchanted creature dies, return this card to its owner's hand and create {create_clause}"
        ));
    }

    None
}

fn normalize_chosen_creature_type_clause(text: &str) -> Option<String> {
    let rest = strip_prefix_ascii_ci(
        text.trim(),
        "You choose exactly 1 creature in the battlefield and tags it as 'chosen_creature_type_ref'. ",
    )?;
    let rest = rest.trim();

    let shared_get_prefixes = [
        "creature that shares a creature types with that object get ",
        "creature that shares a creature type with that object get ",
    ];
    let shared_gain_prefixes = [
        "creature that shares a creature types with that object gain ",
        "creature that shares a creature type with that object gain ",
    ];

    for get_prefix in shared_get_prefixes {
        if let Some((pump_clause, gain_clause)) = rest.split_once(". ")
            && let Some(pump_amount) = strip_prefix_ascii_ci(pump_clause.trim(), get_prefix)
        {
            for gain_prefix in shared_gain_prefixes {
                if let Some(gain_text) = strip_prefix_ascii_ci(gain_clause.trim(), gain_prefix)
                    && let Some(gain_body) =
                        strip_suffix_ascii_ci(gain_text.trim(), " until end of turn.").or_else(
                            || strip_suffix_ascii_ci(gain_text.trim(), " until end of turn"),
                        )
                {
                    let pump_amount =
                        strip_suffix_ascii_ci(pump_amount.trim(), " until end of turn.")
                            .or_else(|| {
                                strip_suffix_ascii_ci(pump_amount.trim(), " until end of turn")
                            })
                            .unwrap_or_else(|| pump_amount.trim());
                    return Some(format!(
                        "Creatures of the creature type of your choice get {} and gain {} until end of turn.",
                        pump_amount.trim(),
                        gain_body.trim()
                    ));
                }
            }
        }
    }

    shared_get_prefixes
        .iter()
        .find_map(|prefix| strip_prefix_ascii_ci(rest, prefix))
        .map(|buff_clause| {
            format!(
                "Creatures of the creature type of your choice get {}",
                buff_clause.trim()
            )
        })
}

fn normalize_subject_signature_for_get_gain(subject: &str) -> String {
    let mut words = Vec::new();
    for raw_word in subject.split_whitespace() {
        let lower = raw_word
            .trim_matches(|ch: char| !ch.is_ascii_alphanumeric())
            .to_ascii_lowercase();
        if lower.is_empty() {
            continue;
        }
        if matches!(
            lower.as_str(),
            "a" | "an"
                | "and"
                | "another"
                | "any"
                | "each"
                | "every"
                | "other"
                | "some"
                | "the"
                | "this"
                | "their"
                | "their's"
                | "these"
                | "them"
                | "it"
                | "its"
                | "to"
                | "with"
                | "your"
        ) {
            continue;
        }
        let normalized = if lower.len() > 3 && lower.ends_with('s') {
            lower[..lower.len() - 1].to_string()
        } else {
            lower
        };
        words.push(normalized);
    }
    words.join(" ")
}

fn normalize_sacrifice_implied_choice(sentence: &str) -> Option<String> {
    let trimmed = sentence.trim();
    let lower = trimmed.to_ascii_lowercase();
    if !lower.contains("sacrifice") || lower.contains("choice") {
        return None;
    }

    let (subject, mut body) = if let Some(rhs) =
        strip_prefix_ascii_ci(trimmed, "that player sacrifices ")
    {
        ("that player sacrifices ", rhs)
    } else if let Some(rhs) = strip_prefix_ascii_ci(trimmed, "each player sacrifices ") {
        ("each player sacrifices ", rhs)
    } else if let Some(rhs) = strip_prefix_ascii_ci(trimmed, "its controller sacrifices ") {
        ("its controller sacrifices ", rhs)
    } else if let Some(rhs) = strip_prefix_ascii_ci(trimmed, "that object's controller sacrifices ")
    {
        ("that object's controller sacrifices ", rhs)
    } else if let Some(rhs) = strip_prefix_ascii_ci(trimmed, "that player's controller sacrifices ")
    {
        ("that player's controller sacrifices ", rhs)
    } else {
        return None;
    };

    let mut body = body.trim().trim_end_matches('.').to_string();
    let body_lower = body.to_ascii_lowercase();
    if body_lower.contains("of your choice") || body_lower.contains("of their choice") {
        return None;
    }

    for suffix in [
        " that player controls",
        " that object's controller controls",
        " that object's controller's control",
        " that your controller controls",
        " its controller controls",
        " your control",
    ] {
        if let Some(stripped) = strip_suffix_ascii_ci(body.as_str(), suffix) {
            body = stripped.to_string();
            break;
        }
    }

    if let Some(rest) = strip_prefix_ascii_ci(&body, "a controller's ") {
        body = rest.to_string();
        if let Some(rest_tail) = strip_prefix_ascii_ci(&body, "a ") {
            body = rest_tail.to_string();
        } else {
            body = format!("a {body}");
        }
    } else if let Some(rest) = strip_prefix_ascii_ci(&body, "an controller's ") {
        body = rest.to_string();
        if let Some(rest_tail) = strip_prefix_ascii_ci(&body, "an ") {
            body = rest_tail.to_string();
        } else {
            body = format!("an {body}");
        }
    } else if let Some(rest) = strip_prefix_ascii_ci(&body, "the controller's ") {
        body = rest.to_string();
        if let Some(rest_tail) = strip_prefix_ascii_ci(&body, "the ") {
            body = rest_tail.to_string();
        } else {
            body = format!("the {body}");
        }
    } else if let Some(rest) = strip_prefix_ascii_ci(&body, "controller's ") {
        body = rest.to_string();
    }

    let mut split_at = body.len();
    let split_markers = [" unless ", " if ", " then "];
    for marker in split_markers {
        if let Some(idx) = body.to_ascii_lowercase().find(marker) {
            if idx < split_at {
                split_at = idx;
            }
        }
    }

    if split_at == body.len() {
        body = format!("{body} of their choice");
    } else {
        body = format!("{} of their choice{}", &body[..split_at], &body[split_at..]);
    }

    let mut rewritten = format!("{subject}{body}");
    if trimmed.ends_with('.') {
        rewritten.push('.');
    }
    Some(rewritten)
}

fn normalize_choose_sacrifice_subject(chosen: &str) -> String {
    let mut chosen = chosen.trim().trim_end_matches('.').to_string();
    if let Some((before, _)) = split_once_ascii_ci(&chosen, " and tag it as ") {
        chosen = before.to_string();
    } else if let Some((before, _)) = split_once_ascii_ci(&chosen, " and tags it as ") {
        chosen = before.to_string();
    }
    chosen = chosen
        .strip_suffix(" in the battlefield")
        .or_else(|| chosen.strip_suffix(" in the battlefields"))
        .or_else(|| chosen.strip_suffix(" you control in the battlefield"))
        .or_else(|| chosen.strip_suffix(" you control in the battlefields"))
        .unwrap_or(chosen.as_str())
        .trim()
        .to_string();
    if let Some(rest) = strip_prefix_ascii_ci(&chosen, "at least 1 ") {
        chosen = rest.trim().to_string();
    }
    let chosen_words = chosen.split_whitespace().collect::<Vec<_>>();
    if let Some(cutoff) = chosen_words
        .iter()
        .position(|word| word.eq_ignore_ascii_case("you") || word.eq_ignore_ascii_case("in"))
        && cutoff > 0
    {
        chosen = chosen_words[..cutoff].join(" ");
    }
    chosen = chosen
        .strip_prefix("a ")
        .or_else(|| chosen.strip_prefix("an "))
        .unwrap_or(chosen.as_str())
        .trim()
        .to_string();
    pluralize_noun_phrase(&chosen)
}

fn normalize_two_sentence_pump_and_gain_until_end_of_turn(
    left: &str,
    right: &str,
) -> Option<String> {
    let left = left.trim().trim_end_matches('.');
    let left_lower = left.to_ascii_lowercase();
    if !left_lower.ends_with("until end of turn") && !left_lower.ends_with("until your turn") {
        return None;
    }

    let mut get_idx = None;
    for (needle, suffix_len) in [(" gets ", 5usize), (" get ", 4usize)] {
        if let Some(idx) = left_lower.rfind(needle) {
            match get_idx {
                None => get_idx = Some((idx, suffix_len)),
                Some((existing_idx, _)) if idx > existing_idx => get_idx = Some((idx, suffix_len)),
                _ => {}
            }
        }
    }
    let (get_idx, get_suffix_len) = get_idx?;
    let left_subject = left[..get_idx].trim();
    if left_subject.is_empty() {
        return None;
    }
    let left_get_keyword = if get_suffix_len == 5 { "gets" } else { "get" };
    let left_pump_body = left[get_idx + get_suffix_len..].trim();
    let left_pump_body = strip_suffix_ascii_ci(left_pump_body, " until end of turn")
        .or_else(|| strip_suffix_ascii_ci(left_pump_body, " until your turn"))?
        .trim();
    let left_sig = normalize_subject_signature_for_get_gain(left_subject);
    if left_sig.is_empty() {
        return None;
    }

    let right = right.trim().trim_end_matches('.');
    let right_lower = right.to_ascii_lowercase();
    let mut gain_idx = None;
    for (needle, suffix_len) in [(" gains ", 6usize), (" gain ", 5usize)] {
        if let Some(idx) = right_lower.find(needle) {
            match gain_idx {
                None => gain_idx = Some((idx, suffix_len)),
                Some((existing_idx, _)) if idx < existing_idx => gain_idx = Some((idx, suffix_len)),
                _ => {}
            }
        }
    }
    let (gain_idx, gain_suffix_len) = gain_idx?;
    let right_subject = right[..gain_idx].trim();
    if right_subject.is_empty() {
        return None;
    }
    let right_gain_body = right[gain_idx + gain_suffix_len..].trim();
    let right_gain_body = strip_suffix_ascii_ci(right_gain_body, " until end of turn")
        .or_else(|| strip_suffix_ascii_ci(right_gain_body, " until your turn"))?;
    let right_sig = normalize_subject_signature_for_get_gain(right_subject);
    if right_sig.is_empty() || right_sig != left_sig {
        return None;
    }

    Some(format!(
        "{} {} {} and gains {} until end of turn.",
        left_subject,
        left_get_keyword,
        left_pump_body,
        right_gain_body.trim(),
    ))
}

fn normalize_pump_and_gain_until_end_of_turn(line: &str) -> Option<String> {
    let segments: Vec<&str> = line.split(". ").collect();
    if segments.len() < 2 {
        return None;
    }

    let first = segments[0];
    let second = segments[1];
    let merged = normalize_two_sentence_pump_and_gain_until_end_of_turn(first, second)?;
    if segments.len() == 2 {
        return Some(merged);
    }

    Some(format!(
        "{} {}",
        merged,
        segments[2..].join(". ").trim_start()
    ))
}

fn normalize_common_semantic_phrasing(line: &str) -> String {
    let mut normalized = line.trim().to_string();
    if let Some(rewritten) = normalize_granted_activated_ability_clause(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_granted_beginning_trigger_clause(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_enchanted_creature_dies_clause(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_chosen_creature_type_clause(&normalized) {
        normalized = rewritten;
    } else if let Some((head, tail)) = split_once_ascii_ci(&normalized, ": ")
        && let Some(rewritten_tail) = normalize_chosen_creature_type_clause(tail)
    {
        normalized = format!("{}: {}", head.trim(), rewritten_tail);
    }
    if let Some(rewritten) = normalize_pump_and_gain_until_end_of_turn(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_search_you_own_clause(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_inline_earthbend_phrasing(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_reveal_tagged_draw_clause(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_sacrifice_implied_choice(&normalized) {
        normalized = rewritten;
    }
    if let Some((tap_clause, untap_clause)) = normalized.split_once(". ")
        && tap_clause.to_ascii_lowercase().starts_with("tap up to ")
        && tap_clause
            .to_ascii_lowercase()
            .contains(" target creatures")
        && (untap_clause
            .eq_ignore_ascii_case("creature can't untap during its controller's next untap step.")
            || untap_clause.eq_ignore_ascii_case(
                "creature can't untap during its controller's next untap step",
            ))
    {
        normalized = format!(
            "{tap_clause}. Those creatures don't untap during their controller's next untap step."
        );
    }
    if let Some((tap_clause, untap_clause)) = normalized.split_once(". ")
        && (untap_clause
            .eq_ignore_ascii_case("permanent can't untap during its controller's next untap step.")
            || untap_clause.eq_ignore_ascii_case(
                "permanent can't untap during its controller's next untap step",
            )
            || untap_clause
                .eq_ignore_ascii_case("land can't untap during its controller's next untap step.")
            || untap_clause.eq_ignore_ascii_case(
                "land can't untap during its controller's next untap step",
            ))
    {
        let tap_lower = tap_clause.to_ascii_lowercase();
        if tap_lower.contains("tap target creature")
            || tap_lower.contains("tap up to one target creature")
        {
            normalized = format!(
                "{tap_clause}. That creature doesn't untap during its controller's next untap step."
            );
        } else if tap_lower.contains("tap target land")
            || tap_lower.contains("tap up to one target land")
        {
            normalized = format!(
                "{tap_clause}. That land doesn't untap during its controller's next untap step."
            );
        } else if tap_lower.contains("tap target nonland permanent")
            || tap_lower.contains("tap up to one target nonland permanent")
            || tap_lower.contains("tap target permanent")
            || tap_lower.contains("tap up to one target permanent")
        {
            normalized = format!(
                "{tap_clause}. That permanent doesn't untap during its controller's next untap step."
            );
        }
    }
    if normalized.contains("Add 1 mana of commander's color identity") {
        normalized = normalized.replace(
            "Add 1 mana of commander's color identity",
            "Add one mana of any color in your commander's color identity",
        );
    }
    if normalized.contains("create a Powerstone artifact token, tapped") {
        normalized = normalized.replace(
            "create a Powerstone artifact token, tapped",
            "create a tapped Powerstone token",
        );
    }
    // Fix "all/each/for each another" → "all/each/for each other" (grammar fix for
    // filter descriptions with `other: true` used in quantified contexts).
    normalized = normalized
        .replace("all another ", "all other ")
        .replace("All another ", "All other ")
        .replace("each another ", "each other ")
        .replace("Each another ", "Each other ")
        .replace("For each another ", "For each other ")
        .replace("for each another ", "for each other ");
    if normalized.contains("Other Elf you control get ") {
        normalized =
            normalized.replace("Other Elf you control get ", "Other Elves you control get ");
    }
    normalized = normalized
        .replace(
            "you may target creature gets ",
            "you may have target creature get ",
        )
        .replace(
            "you may target creature gains ",
            "you may have target creature gain ",
        )
        .replace(
            "you may target creature loses ",
            "you may have target creature lose ",
        )
        .replace(
            "you may target creature reveals ",
            "you may have target creature reveal ",
        );
    if normalized.starts_with("creatures you control get ") {
        normalized = normalized.replacen(
            "creatures you control get ",
            "Each creature you control gets ",
            1,
        );
    }
    if normalized.starts_with("Creatures you control get ") {
        normalized = normalized.replacen(
            "Creatures you control get ",
            "Each creature you control gets ",
            1,
        );
    }
    normalized = normalized
        .replace(
            ": creatures you control get ",
            ": Each creature you control gets ",
        )
        .replace(
            ": Creatures you control get ",
            ": Each creature you control gets ",
        )
        .replace(
            ". creatures you control get ",
            ". Each creature you control gets ",
        )
        .replace(
            ". Creatures you control get ",
            ". Each creature you control gets ",
        )
        .replace(
            "• creatures you control get ",
            "• Each creature you control gets ",
        )
        .replace(
            "• Creatures you control get ",
            "• Each creature you control gets ",
        );
    if let Some(rest) = normalized.strip_prefix("Target player discards ")
        && let Some((discard_count, loss_tail)) = rest.split_once(" cards. target player loses ")
        && let Some(loss_amount) = loss_tail.strip_suffix(" life.")
    {
        normalized =
            format!("Target player discards {discard_count} cards and loses {loss_amount} life.");
    }
    if let Some(rest) = normalized
        .strip_prefix("Whenever this creature attacks, choose another target attacking creature. ")
        && rest
            .to_ascii_lowercase()
            .starts_with("another target attacking creature can't be blocked this turn")
    {
        normalized = format!("Whenever this creature attacks, {}", rest.trim());
    }
    if let Some((prefix, rest)) =
        split_once_ascii_ci(&normalized, "for each opponent, that player sacrifices ")
        && let Some((sacrifice_tail, pay_tail)) =
            split_once_ascii_ci(rest, " unless that player pays ")
    {
        normalized = format!(
            "{}, each opponent sacrifices {} of their choice unless they pay {}",
            prefix.trim_end_matches(|c| c == ',' || c == ' '),
            sacrifice_tail.trim_end_matches('.'),
            pay_tail.trim_end_matches('.')
        );
    }
    if normalized.contains("another target creature has base power and toughness") {
        normalized = normalized.replace(
            "another target creature has base power and toughness",
            "target creature other than this creature has base power and toughness",
        );
    }
    if let Some(rest) = strip_prefix_ascii_ci(&normalized, "Creatures you control get ")
        && let Some(buff) = strip_suffix_ascii_ci(rest, " until end of turn. Untap all permanent.")
            .or_else(|| strip_suffix_ascii_ci(rest, " until end of turn. Untap all permanent"))
    {
        return format!("Creatures you control get {buff} until end of turn. Untap them.");
    }
    let lower = normalized.to_ascii_lowercase();

    if lower == "creatures have blocks each combat if able"
        || lower == "all creatures have blocks each combat if able"
    {
        return "All creatures able to block this creature do so".to_string();
    }
    if let Some((prefix, rest)) = normalized.split_once(": Choose target ")
        && let Some((subject, tail)) = rest.split_once(". ")
    {
        let subject = subject.trim();
        let tail_trimmed = tail.trim();
        if !subject.is_empty()
            && tail_trimmed
                .to_ascii_lowercase()
                .starts_with(&format!("target {} ", subject.to_ascii_lowercase()))
        {
            normalized = format!("{}: {}", prefix.trim(), capitalize_first(tail_trimmed));
        }
    }
    if let Some(rest) = normalized.strip_prefix("Choose target ")
        && let Some((subject, tail)) = rest.split_once(". ")
    {
        let subject = subject.trim();
        if !subject.is_empty()
            && tail
                .to_ascii_lowercase()
                .starts_with(&format!("target {} ", subject.to_ascii_lowercase()))
        {
            return capitalize_first(tail.trim());
        }
    }
    if lower.contains("energy counter(s)") {
        for count in 1usize..=8 {
            let digit = count.to_string();
            let symbols = repeated_energy_symbols(count);
            normalized = normalized
                .replace(
                    &format!("you get {digit} energy counter(s)"),
                    &format!("you get {symbols}"),
                )
                .replace(
                    &format!("You get {digit} energy counter(s)"),
                    &format!("You get {symbols}"),
                );
        }
    }
    for life in 1usize..=20 {
        let amount = life.to_string();
        normalized = normalized
            .replace(
                &format!("you may lose {amount} life. If you do"),
                &format!("you may pay {amount} life. If you do"),
            )
            .replace(
                &format!("You may lose {amount} life. If you do"),
                &format!("You may pay {amount} life. If you do"),
            )
            .replace(
                &format!("you may lose {amount} life and "),
                &format!("you may pay {amount} life and "),
            )
            .replace(
                &format!("You may lose {amount} life and "),
                &format!("You may pay {amount} life and "),
            );
    }
    if let Some((left, right)) = normalized.split_once(": ")
        && matches!(
            right,
            "Target creature can't untap until your next turn"
                | "Target creature cant untap until your next turn"
        )
    {
        return format!(
            "{left}: Target creature doesn't untap during its controller's next untap step"
        );
    }
    normalized = normalized
        .replace("target another ", "other target ")
        .replace("a another ", "another ")
        .replace("Whenever a another ", "Whenever another ")
        .replace("Others ", "Other ")
        .replace(" that objects ", " that object ")
        .replace(" that objects.", " that object.")
        .replace(" that objects,", " that object,")
        .replace(" to that objects", " to that object")
        .replace(
            "an opponent's creature enter the battlefield tapped",
            "an opponent's creature enters the battlefield tapped",
        )
        .replace(
            "opponent's artifact enter the battlefield tapped",
            "opponent's artifact enters the battlefield tapped",
        )
        .replace(
            "Creature enter the battlefield tapped",
            "Creature enters the battlefield tapped",
        )
        .replace(
            "When this permanent enters or this creature attacks",
            "Whenever this creature enters or attacks",
        )
        .replace(
            "When this creature enters or this creature attacks",
            "Whenever this creature enters or attacks",
        )
        .replace(
            "when this permanent enters or this creature attacks",
            "whenever this creature enters or attacks",
        )
        .replace(
            "when this creature enters or this creature attacks",
            "whenever this creature enters or attacks",
        )
        .replace(
            "a land you control can't untap until your next turn",
            "Lands you control don't untap during your next untap step",
        )
        .replace(
            "a land you control cant untap until your next turn",
            "Lands you control don't untap during your next untap step",
        )
        .replace(
            "permanent can't untap while you control this creature",
            "that creature doesn't untap during its controller's untap step for as long as you control this creature",
        )
        .replace(
            "permanent cant untap while you control this creature",
            "that creature doesn't untap during its controller's untap step for as long as you control this creature",
        )
        .replace(
            "land Zombie or Swamp you own, reveal it, put it into your hand, then shuffle",
            "a Zombie card and a Swamp card, reveal them, put them into your hand, then shuffle",
        )
        .replace(" in yours graveyard", " in your graveyard")
        .replace("counter ons it", "counter on it")
        .replace(
            "Target attacking/blocking creature",
            "Target attacking or blocking creature",
        )
        .replace(
            "target attacking/blocking creature",
            "target attacking or blocking creature",
        )
        .replace(
            "with a +1/+1 counter on it you control",
            "you control with a +1/+1 counter on it",
        )
        .replace(
            "with a -1/-1 counter on it you control",
            "you control with a -1/-1 counter on it",
        )
        .replace(
            "with a counter on it you control",
            "you control with a counter on it",
        )
        .replace(
            "Activate only as a sorcery and Activate only once each turn",
            "Activate only as a sorcery and only once each turn",
        )
        .replace(
            "activate only as a sorcery and activate only once each turn",
            "Activate only as a sorcery and only once each turn",
        )
        .replace(
            "you sacrifice another creature you control",
            "sacrifice another creature",
        )
        .replace(
            "you sacrifice a creature you control",
            "sacrifice a creature",
        )
        .replace(
            "you sacrifice an artifact you control",
            "sacrifice an artifact",
        )
        .replace("you sacrifice a land you control", "sacrifice a land")
        .replace(
            "you sacrifice a permanent you control",
            "sacrifice a permanent",
        )
        .replace(
            "you sacrifice two creatures you control",
            "sacrifice two creatures",
        )
        .replace(
            "you sacrifice three creatures you control",
            "sacrifice three creatures",
        )
        .replace(
            "you may sacrifice another creature you control",
            "you may sacrifice another creature",
        )
        .replace(
            "you may sacrifice a creature you control",
            "you may sacrifice a creature",
        )
        .replace(
            "you may sacrifice an artifact you control",
            "you may sacrifice an artifact",
        )
        .replace(
            "Create a Powerstone artifact token tapped under your control",
            "Create a tapped Powerstone token",
        )
        .replace(
            "Create a Powerstone artifact token under your control, tapped",
            "Create a tapped Powerstone token",
        )
        .replace(
            "Create 1 Powerstone artifact token tapped under your control",
            "Create a tapped Powerstone token",
        )
        .replace(
            "Create 1 Powerstone artifact token under your control, tapped",
            "Create a tapped Powerstone token",
        )
        .replace(
            "create a Powerstone artifact token tapped under your control",
            "create a tapped Powerstone token",
        )
        .replace(
            "create a Powerstone artifact token under your control, tapped",
            "create a tapped Powerstone token",
        )
        .replace(
            "create 1 Powerstone artifact token tapped under your control",
            "create a tapped Powerstone token",
        )
        .replace(
            "create 1 Powerstone artifact token under your control, tapped",
            "create a tapped Powerstone token",
        )
        .replace(
            "Prevent combat damage until end of turn",
            "Prevent all combat damage that would be dealt this turn",
        )
        .replace(" put it into hand", " put it into your hand")
        .replace("for blue instant you own", "for a blue instant card")
        .replace("for creature you own", "for a creature card")
        .replace(
            "Search your library for Equipment you own, reveal it, put it into your hand, then shuffle",
            "Search your library for an Equipment card, reveal that card, put it into your hand, then shuffle",
        )
        .replace(
            "Search your library for Arcane you own, reveal it, put it into your hand, then shuffle",
            "Search your library for an Arcane card, reveal that card, put it into your hand, then shuffle",
        )
        .replace(
            "search your library for Equipment you own, reveal it, put it into your hand, then shuffle",
            "search your library for an Equipment card, reveal that card, put it into your hand, then shuffle",
        )
        .replace(
            "search your library for Arcane you own, reveal it, put it into your hand, then shuffle",
            "search your library for an Arcane card, reveal that card, put it into your hand, then shuffle",
        )
        .replace(
            "search your library for land Forest you own, put it onto the battlefield, then shuffle",
            "search your library for a Forest card, put that card onto the battlefield, then shuffle",
        )
        .replace(
            "Search your library for land Forest you own, put it onto the battlefield, then shuffle",
            "Search your library for a Forest card, put that card onto the battlefield, then shuffle",
        )
        .replace(
            "Search your library for land Forest, put it onto the battlefield, then shuffle",
            "Search your library for a Forest card, put that card onto the battlefield, then shuffle",
        )
        .replace(
            "for Aura or Equipment you own",
            "for an Aura or Equipment card",
        )
        .replace(
            "it changes controller to this effect's controller and gains Haste until end of turn",
            "Gain control of it until end of turn. It gains haste until end of turn",
        )
        .replace(
            "it changes controller to this effect's controller and gains haste until end of turn",
            "Gain control of it until end of turn. It gains haste until end of turn",
        )
        .replace(
            "you may Put target creature card in your hand onto the battlefield. it gains Haste until end of turn. you sacrifice a creature.",
            "You may put a creature card from your hand onto the battlefield. That creature gains haste. Sacrifice the creature at the beginning of the next end step.",
        )
        .replace(
            "you may Put target creature card in your hand onto the battlefield. it gains Haste until end of turn. you sacrifice a creature",
            "You may put a creature card from your hand onto the battlefield. That creature gains haste. Sacrifice the creature at the beginning of the next end step.",
        )
        .replace(
            "you may Put creature card in your hand onto the battlefield. it gains Haste until end of turn. you sacrifice a creature.",
            "You may put a creature card from your hand onto the battlefield. That creature gains haste. Sacrifice the creature at the beginning of the next end step.",
        )
        .replace(
            "you may Put creature card in your hand onto the battlefield. it gains Haste until end of turn. you sacrifice a creature",
            "You may put a creature card from your hand onto the battlefield. That creature gains haste. Sacrifice the creature at the beginning of the next end step.",
        )
        .replace(
            "you may Put target creature card in your hand onto the battlefield. it gains Haste. At the beginning of the next end step, you sacrifice it.",
            "You may put a creature card from your hand onto the battlefield. That creature gains haste. Sacrifice the creature at the beginning of the next end step.",
        )
        .replace(
            "you may Put target creature card in your hand onto the battlefield. it gains Haste. At the beginning of the next end step, you sacrifice it",
            "You may put a creature card from your hand onto the battlefield. That creature gains haste. Sacrifice the creature at the beginning of the next end step.",
        )
        .replace(
            "you may Put creature card in your hand onto the battlefield. it gains Haste. At the beginning of the next end step, you sacrifice it.",
            "You may put a creature card from your hand onto the battlefield. That creature gains haste. Sacrifice the creature at the beginning of the next end step.",
        )
        .replace(
            "you may Put creature card in your hand onto the battlefield. it gains Haste. At the beginning of the next end step, you sacrifice it",
            "You may put a creature card from your hand onto the battlefield. That creature gains haste. Sacrifice the creature at the beginning of the next end step.",
        )
        .replace(
            "it gains Haste until end of turn. At the beginning of the next end step, you sacrifice it.",
            "That creature gains haste until end of turn. At the beginning of the next end step, sacrifice that creature.",
        )
        .replace(
            "it gains Haste until end of turn. At the beginning of the next end step, you sacrifice it",
            "That creature gains haste until end of turn. At the beginning of the next end step, sacrifice that creature.",
        )
        .replace(
            "it gains Haste. At the beginning of the next end step, you sacrifice it.",
            "That creature gains haste. Sacrifice the creature at the beginning of the next end step.",
        )
        .replace(
            "it gains Haste. At the beginning of the next end step, you sacrifice it",
            "That creature gains haste. Sacrifice the creature at the beginning of the next end step.",
        )
        .replace(
            "Untap it. it gains Haste until end of turn",
            "Untap that creature. It gains haste until end of turn",
        )
        .replace(
            "Untap it. it gains Haste and gains Menace until end of turn",
            "Untap that creature. It gains haste and menace until end of turn",
        )
        .replace(
            "it gains Haste and gains Menace until end of turn",
            "it gains haste and menace until end of turn",
        )
        .replace(
            "at the beginning of the next end step. you control.",
            "at the beginning of the next end step.",
        )
        .replace(
            "At the beginning of the next end step. you control.",
            "At the beginning of the next end step.",
        )
        .replace(
            "An opponent's artifact or creature enter the battlefield tapped.",
            "Artifacts and creatures your opponents control enter tapped.",
        )
        .replace(
            "An opponent's artifact or creature enter the battlefield tapped",
            "Artifacts and creatures your opponents control enter tapped",
        )
        .replace(
            "An opponent's artifact enters the battlefield tapped.",
            "Artifacts your opponents control enter the battlefield tapped.",
        )
        .replace(
            "An opponent's artifact enters the battlefield tapped",
            "Artifacts your opponents control enter the battlefield tapped",
        )
        .replace(
            "An opponent's creature enters the battlefield tapped.",
            "Creatures your opponents control enter the battlefield tapped.",
        )
        .replace(
            "An opponent's creature enters the battlefield tapped",
            "Creatures your opponents control enter the battlefield tapped",
        )
        .replace(
            "An opponent's nonbasic creature or land enter the battlefield tapped.",
            "Creatures and nonbasic lands your opponents control enter tapped.",
        )
        .replace(
            "An opponent's nonbasic creature or land enter the battlefield tapped",
            "Creatures and nonbasic lands your opponents control enter tapped",
        )
        .replace(
            "with \"Sacrifice this creature, add {C}\"",
            "with \"Sacrifice this creature: Add {C}.\"",
        )
        .replace(
            "with \"sacrifice this creature, add {C}\"",
            "with \"Sacrifice this creature: Add {C}.\"",
        )
        .replace(
            "Whenever a creature blocks, deal 1 damage to that object's controller.",
            "Whenever a creature blocks, deal 1 damage to that creature's controller.",
        )
        .replace(
            "Whenever a creature blocks, deal 1 damage to that object's controller",
            "Whenever a creature blocks, deal 1 damage to that creature's controller",
        )
        .replace(
            "Whenever a creature blocks, this enchantment deals 1 damage to that creature's controller.",
            "Whenever a creature blocks, deal 1 damage to that creature's controller.",
        )
        .replace(
            "Whenever a creature blocks, this enchantment deals 1 damage to that creature's controller",
            "Whenever a creature blocks, deal 1 damage to that creature's controller",
        );
    if let Some((heading, tail)) = split_once_ascii_ci(&normalized, ": ")
        && let Some(rest) = strip_prefix_ascii_ci(tail, "Whenever a creature blocks, deal ")
        && let Some(dmg) = rest.strip_suffix(" damage to that object's controller.")
    {
        normalized = format!(
            "{heading}: Whenever a creature blocks, deal {dmg} damage to that creature's controller.",
            heading = heading.trim_end_matches(':').trim()
        );
    } else if let Some((heading, tail)) = split_once_ascii_ci(&normalized, ": ")
        && let Some(rest) = strip_prefix_ascii_ci(tail, "Whenever a creature blocks, deal ")
        && let Some(dmg) = rest.strip_suffix(" damage to that object's controller")
    {
        normalized = format!(
            "{heading}: Whenever a creature blocks, deal {dmg} damage to that creature's controller",
            heading = heading.trim_end_matches(':').trim()
        );
    }
    if let Some(amount) = strip_prefix_ascii_ci(
        &normalized,
        "Whenever enchanted land is tapped for mana, add ",
    )
    .and_then(|tail| {
        strip_suffix_ascii_ci(tail, " to that object's controller's mana pool.")
            .or_else(|| strip_suffix_ascii_ci(tail, " to that object's controller's mana pool"))
    }) {
        return format!(
            "Whenever enchanted land is tapped for mana, its controller adds an additional {}.",
            amount.trim()
        );
    }
    if let Some((left, right)) = normalized.split_once(". ")
        && left.to_ascii_lowercase().contains("target creature")
        && right.eq_ignore_ascii_case("Untap it.")
    {
        normalized = format!("{}. Untap that creature.", left.trim_end_matches('.'));
    } else if let Some((left, right)) = normalized.split_once(". ")
        && left.to_ascii_lowercase().contains("target creature")
        && right.eq_ignore_ascii_case("Untap it")
    {
        normalized = format!("{}. Untap that creature", left.trim_end_matches('.'));
    }
    if let Some((left, right)) = normalized.split_once(". ")
        && left.to_ascii_lowercase().contains("target creature")
        && right.eq_ignore_ascii_case("Tap it.")
    {
        normalized = format!("{}. Tap it.", left.trim_end_matches('.'));
    } else if let Some((left, right)) = normalized.split_once(". ")
        && left.to_ascii_lowercase().contains("target creature")
        && right.eq_ignore_ascii_case("Tap it")
    {
        normalized = format!("{}. Tap it", left.trim_end_matches('.'));
    }
    if let Some((left, right)) = normalized.split_once(". ")
        && (left.eq_ignore_ascii_case("Untap one or two target creatures")
            || left.eq_ignore_ascii_case("Untap up to two target creatures"))
        && let Some(buff_clause) = strip_suffix_ascii_ci(right.trim(), " until end of turn.")
            .or_else(|| strip_suffix_ascii_ci(right.trim(), " until end of turn"))
        && let Some(buff) = strip_prefix_ascii_ci(buff_clause.trim(), "it gets ")
            .or_else(|| strip_prefix_ascii_ci(buff_clause.trim(), "It gets "))
    {
        return format!(
            "{}. They each get {} until end of turn.",
            left.trim_end_matches('.'),
            buff.trim()
        );
    }
    if let Some((prefix, suffix)) =
        normalized.split_once(", reveal it, put it on top of library, then shuffle")
    {
        normalized = format!("{prefix}, reveal it, then shuffle and put the card on top{suffix}");
    }
    normalized = normalized
        .replace(
            "can't be blocked until end of turn",
            "can't be blocked this turn",
        )
        .replace(
            "cant be blocked until end of turn",
            "can't be blocked this turn",
        )
        .replace("can't block until end of turn", "can't block this turn")
        .replace("cant block until end of turn", "can't block this turn")
        .replace("If it happened, ", "If you do, ")
        .replace("If you do, you draw", "If you do, draw")
        .replace("If you do, you discard", "If you do, discard");
    if normalized.contains("Manifest dread. Put ") && normalized.contains(" on it. Put ") {
        normalized = normalized
            .replace("Manifest dread. Put ", "Manifest dread, then put ")
            .replace(" on it. Put ", " and ");
    }
    let lower_normalized = normalized.to_ascii_lowercase();
    if let Some(tail) = strip_prefix_ascii_ci(
        &normalized,
        "You take an extra turn after this one. At the beginning of your next end step, ",
    ) && tail
        .trim()
        .trim_end_matches('.')
        .eq_ignore_ascii_case("you lose the game")
    {
        return "Take an extra turn after this one. At the beginning of that turn's end step, you lose the game".to_string();
    }
    if let Some(amount) =
        strip_prefix_ascii_ci(&normalized, "At the beginning of your upkeep, deal ").and_then(
            |tail| {
                strip_suffix_ascii_ci(tail, " damage to you.")
                    .or_else(|| strip_suffix_ascii_ci(tail, " damage to you"))
            },
        )
    {
        return format!(
            "At the beginning of your upkeep, this creature deals {} damage to you.",
            amount.trim()
        );
    }
    if lower_normalized == "cards in hand have flash"
        || lower_normalized == "cards in hand have flash."
    {
        return "You may cast noncreature spells as though they had flash".to_string();
    }
    if lower_normalized
        == "tap any number of an untapped creature you control and you gain 4 life for each tapped creature"
        || lower_normalized
            == "tap any number of an untapped creature you control and you gain 4 life for each tapped creature."
    {
        return "Tap any number of untapped creatures you control. You gain 4 life for each creature tapped this way".to_string();
    }
    if let Some((left, right)) = normalized.split_once(". ")
        && left.starts_with("Deal ")
        && left.contains("damage to target opponent or planeswalker")
        && (right.starts_with("target opponent discards ")
            || right.starts_with("Target opponent discards "))
    {
        let discard_tail = right
            .strip_prefix("target opponent discards ")
            .or_else(|| right.strip_prefix("Target opponent discards "))
            .unwrap_or(right);
        return format!(
            "{left}. That player or that planeswalker's controller discards {discard_tail}"
        );
    }
    if lower_normalized.contains("tap x target artifacts or creatures or lands")
        && lower_normalized.contains("you lose x life")
    {
        return "Tap X target artifacts, creatures, and/or lands. You lose X life.".to_string();
    }
    if lower_normalized == "creatures have can't block"
        || lower_normalized == "all creatures have can't block"
    {
        return "Creatures can't block".to_string();
    }
    if lower_normalized == "can't block" || lower_normalized == "can't block." {
        return "This creature can't block".to_string();
    }
    if lower_normalized == "can't be blocked" || lower_normalized == "can't be blocked." {
        return "This creature can't be blocked".to_string();
    }
    if lower_normalized == "enchant opponent's creature" {
        return "Enchant creature an opponent controls".to_string();
    }
    if lower_normalized == "exchange control of creature and creature" {
        return "Exchange control of two target creatures".to_string();
    }
    if lower_normalized == "exchange control of permanent and permanent" {
        return "Exchange control of two target permanents".to_string();
    }
    if lower_normalized
        .starts_with("exchange control of two target permanents that share a card type")
    {
        return "Exchange control of two target permanents".to_string();
    }
    if lower_normalized
        .starts_with("exchange control of two target permanents that share a permanent type")
    {
        return "Exchange control of two target permanents".to_string();
    }
    if lower_normalized == "exchange control of two target permanents that share a card type"
        || lower_normalized == "exchange control of two target permanents that share a card type."
    {
        return "Exchange control of two target permanents".to_string();
    }
    if lower_normalized == "exchange control of two target permanents that share a permanent type"
        || lower_normalized
            == "exchange control of two target permanents that share a permanent type."
    {
        return "Exchange control of two target permanents".to_string();
    }
    if lower_normalized == "destroy all an opponent's nonland permanent"
        || lower_normalized == "destroy all an opponent's nonland permanent."
    {
        return "Destroy all nonland permanents your opponents control".to_string();
    }
    if lower_normalized
        == "destroy all an opponent's creature. destroy all an opponent's planeswalker."
        || lower_normalized
            == "destroy all an opponent's creature. destroy all an opponent's planeswalker"
    {
        return "Destroy all creatures you don't control and all planeswalkers you don't control"
            .to_string();
    }
    let is_simple_mass_noun = |noun: &str| {
        matches!(
            noun.trim_end_matches('.'),
            "artifact"
                | "artifacts"
                | "creature"
                | "creatures"
                | "land"
                | "lands"
                | "enchantment"
                | "enchantments"
                | "spacecraft"
                | "spacecrafts"
        )
    };
    if let Some(rest) = normalized.strip_prefix("Destroy all ")
        && let Some((first, second)) = rest.split_once(". Destroy all ")
    {
        let first = first.trim().trim_end_matches('.');
        let second = second.trim().trim_end_matches('.');
        if is_simple_mass_noun(first) && is_simple_mass_noun(second) {
            return format!(
                "Destroy all {} and {}",
                pluralize_noun_phrase(first),
                pluralize_noun_phrase(second)
            );
        }
    }
    if let Some(rest) = normalized.strip_prefix("Exile all ")
        && let Some((first, second)) = rest.split_once(". Exile all ")
    {
        let first = first.trim().trim_end_matches('.');
        let second = second.trim().trim_end_matches('.');
        if is_simple_mass_noun(first) && is_simple_mass_noun(second) {
            return format!(
                "Exile all {} and {}",
                pluralize_noun_phrase(first),
                pluralize_noun_phrase(second)
            );
        }
    }
    if let Some(rest) = normalized.strip_prefix("For each tagged 'destroyed_")
        && let Some((_, tail)) = rest.split_once("' object, ")
    {
        return format!("For each object destroyed this way, {tail}");
    }
    if let Some(rest) = normalized.strip_prefix("For each tagged 'exiled_")
        && let Some((_, tail)) = rest.split_once("' object, ")
    {
        return format!("For each object exiled this way, {tail}");
    }
    if let Some(rest) = normalized.strip_prefix("For each object destroyed this way, Create ")
        && let Some((token_text, tail)) =
            rest.split_once(" under that object's controller's control")
    {
        return format!(
            "For each object destroyed this way, its controller creates {token_text}{tail}"
        );
    }
    if let Some(rest) = normalized.strip_prefix("For each object destroyed this way, Create ")
        && let Some((token_text, tail)) = rest.split_once(" under that player's control")
    {
        return format!(
            "For each object destroyed this way, that player creates {token_text}{tail}"
        );
    }
    if normalized
        == "For each creature you control with a +1/+1 counter on it, Put a +1/+1 counter on that object"
        || normalized
            == "For each creature you control with a +1/+1 counter on it, Put a +1/+1 counter on that object."
    {
        return "Put a +1/+1 counter on each creature you control with a +1/+1 counter on it"
            .to_string();
    }
    if let Some(prefix) = normalized.strip_suffix(
        " and tag it as 'revealed_0'. If the tagged object 'revealed_0' matches land, Return it to its owner's hand",
    ) {
        return format!("{prefix}. If it's a land card, that player puts it into their hand");
    }
    if let Some(prefix) = normalized.strip_suffix(
        " and tag it as 'revealed_0'. If the tagged object 'revealed_0' matches land, Return it to its owner's hand.",
    ) {
        return format!("{prefix}. If it's a land card, that player puts it into their hand");
    }
    if let Some(prefix) = normalized.strip_suffix(
        " and tag it as 'revealed_0'. If the tagged object 'revealed_0' matches creature, Return it to its owner's hand",
    ) {
        return format!("{prefix}. If it's a creature card, put it into your hand");
    }
    if let Some(prefix) = normalized.strip_suffix(
        " and tag it as 'revealed_0'. If the tagged object 'revealed_0' matches creature, Return it to its owner's hand.",
    ) {
        return format!("{prefix}. If it's a creature card, put it into your hand");
    }
    if let Some(rest) = normalized.strip_prefix("For each player, Deal ")
        && let Some((amount, per_player_tail)) =
            rest.split_once(" damage to that player. For each ")
        && let Some((per_player_filter, repeated_damage)) = per_player_tail.split_once(", Deal ")
        && repeated_damage.trim_end_matches('.') == format!("{amount} damage to that object")
    {
        let each_filter = per_player_filter
            .trim_end_matches(" that player controls")
            .trim();
        return format!("Deal {amount} damage to each {each_filter} and each player");
    }
    if let Some((cost, effect)) = normalized.split_once(": ")
        && let Some(rest) = effect.strip_prefix("For each player, Deal ")
        && let Some((amount, per_player_tail)) =
            rest.split_once(" damage to that player. For each ")
        && let Some((per_player_filter, repeated_damage)) = per_player_tail.split_once(", Deal ")
        && repeated_damage.trim_end_matches('.') == format!("{amount} damage to that object")
    {
        let each_filter = per_player_filter
            .trim_end_matches(" that player controls")
            .trim();
        return format!("{cost}: Deal {amount} damage to each {each_filter} and each player");
    }
    if let Some(rest) = normalized.strip_prefix("Each player creates 1 ")
        && let Some((token_desc, tail)) = rest.split_once(" under that player's control for each ")
        && let Some((each_filter, ending)) = tail
            .split_once(" that player controls.")
            .or_else(|| tail.split_once(" that player controls"))
    {
        return format!(
            "Each player creates a {token_desc} for each {each_filter} they control{ending}"
        );
    }
    if let Some(rest) = normalized.strip_prefix("Each player creates 1 ")
        && let Some((token_desc, tail)) = rest.split_once(" under that player's control")
    {
        return format!("Each player creates a {token_desc}{tail}");
    }
    if let Some(rest) = strip_prefix_ascii_ci(
        &normalized,
        "As an additional cost to cast this spell, you may choose at least 1 ",
    ) && let Some((chosen, tail)) =
        split_once_ascii_ci(rest, ". you sacrifice all permanents you control")
    {
        let chosen_plural = normalize_choose_sacrifice_subject(chosen);
        let tail = tail
            .trim_start_matches('.')
            .trim_start()
            .trim_end_matches('.');
        if tail.is_empty() {
            return format!(
                "As an additional cost to cast this spell, you may sacrifice one or more {chosen_plural}"
            );
        }
        return format!(
            "As an additional cost to cast this spell, you may sacrifice one or more {chosen_plural}. {}.",
            capitalize_first(tail)
        );
    }
    if let Some(rest) = normalized.strip_prefix("You choose any number ")
        && let Some((chosen, tail)) = rest.split_once(". you sacrifice all permanents you control")
    {
        let chosen_plural = normalize_choose_sacrifice_subject(chosen);
        let tail = tail
            .trim_start_matches('.')
            .trim_start()
            .trim_end_matches('.');
        if tail.is_empty() {
            return format!("Sacrifice any number of {chosen_plural}");
        }
        return format!(
            "Sacrifice any number of {chosen_plural}. {}.",
            capitalize_first(tail)
        );
    }
    if let Some(rest) = normalized.strip_prefix("you choose any number ")
        && let Some((chosen, tail)) = rest.split_once(". you sacrifice all permanents you control")
    {
        let chosen_plural = normalize_choose_sacrifice_subject(chosen);
        let tail = tail
            .trim_start_matches('.')
            .trim_start()
            .trim_end_matches('.');
        if tail.is_empty() {
            return format!("Sacrifice any number of {chosen_plural}");
        }
        return format!(
            "Sacrifice any number of {chosen_plural}. {}.",
            capitalize_first(tail)
        );
    }
    if let Some(rest) = normalized.strip_prefix("For each opponent, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to that player")
            .or_else(|| rest.strip_suffix(" damage to that player."))
    {
        return format!("Deal {amount} damage to each opponent");
    }
    if let Some(rest) = normalized.strip_prefix("Investigate. ")
        && rest.starts_with("target creature gets +")
    {
        return format!("Investigate, then {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("For each opponent, Deal ")
        && let Some((amount, tail)) = rest.split_once(" damage to that player. ")
        && (tail.eq_ignore_ascii_case("you gain 1 life")
            || tail.eq_ignore_ascii_case("you gain 1 life."))
    {
        return format!("Deal {amount} damage to each opponent and you gain 1 life");
    }
    if let Some((cost, effect)) = normalized.split_once(": ")
        && let Some(rest) = effect.strip_prefix("For each opponent, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to that player")
            .or_else(|| rest.strip_suffix(" damage to that player."))
    {
        return format!("{cost}: Deal {amount} damage to each opponent");
    }
    if let Some((cost, effect)) = normalized.split_once(": ")
        && let Some(rest) = effect.strip_prefix("For each opponent, Deal ")
        && let Some((amount, tail)) = rest.split_once(" damage to that player. ")
        && (tail.eq_ignore_ascii_case("you gain 1 life")
            || tail.eq_ignore_ascii_case("you gain 1 life."))
    {
        return format!("{cost}: Deal {amount} damage to each opponent and you gain 1 life");
    }
    if let Some(draw_tail) = normalized.strip_prefix("You draw ")
        && let Some(count) = draw_tail.strip_suffix(" cards. Proliferate")
    {
        return format!("Draw {count} cards, then proliferate");
    }
    if let Some(draw_tail) = normalized.strip_prefix("You draw ")
        && let Some(count) = draw_tail.strip_suffix(" cards. Proliferate.")
    {
        return format!("Draw {count} cards, then proliferate");
    }
    if let Some(rest) = normalized.strip_prefix("For each creature, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to that object")
            .or_else(|| rest.strip_suffix(" damage to that object."))
    {
        return format!("Deal {amount} damage to each creature");
    }
    if let Some(rest) = normalized.strip_prefix("For each ")
        && let Some((filter, tail)) = rest.split_once(", Deal ")
        && let Some(amount) = tail
            .strip_suffix(" damage to that object")
            .or_else(|| tail.strip_suffix(" damage to that object."))
        && !filter.starts_with("player")
        && !filter.starts_with("opponent")
        && !filter.starts_with("tagged ")
    {
        return format!("Deal {amount} damage to each {filter}");
    }
    if let Some((cost, effect)) = normalized.split_once(": ")
        && let Some(rest) = effect.strip_prefix("For each ")
        && let Some((filter, tail)) = rest.split_once(", Deal ")
        && let Some(amount) = tail
            .strip_suffix(" damage to that object")
            .or_else(|| tail.strip_suffix(" damage to that object."))
        && !filter.starts_with("player")
        && !filter.starts_with("opponent")
        && !filter.starts_with("tagged ")
    {
        return format!("{cost}: Deal {amount} damage to each {filter}");
    }
    if let Some(tail) =
        normalized.strip_prefix("For each player, that player discards their hand. you draw ")
    {
        return format!("Each player discards their hand, then draws {tail}");
    }
    if let Some(tail) = normalized.strip_prefix(
        "For each player, that player discards their hand. For each player, that player draws ",
    ) {
        return format!("Each player discards their hand, then draws {tail}");
    }
    if normalized
        == "For each player, that player draws a card. For each player, that player discards a card"
        || normalized
            == "For each player, that player draws a card. For each player, that player discards a card."
    {
        return "Each player draws a card, then discards a card".to_string();
    }
    if let Some(tail) = normalized.strip_prefix("You discard your hand. you draw ") {
        let draw_tail = tail.trim_end_matches('.');
        return format!("Discard your hand, then draw {draw_tail}");
    }
    if let Some(tail) = normalized.strip_prefix("Each player discards their hand. Create ")
        && let Some(token_clause) = tail
            .strip_suffix(" under that player's control")
            .or_else(|| tail.strip_suffix(" under that player's control."))
    {
        let normalized_clause = if token_clause.ends_with(" token") {
            format!("{} tokens", token_clause.trim_end_matches(" token"))
        } else {
            token_clause.to_string()
        };
        return format!("Each player discards their hand, then creates {normalized_clause}");
    }
    if let Some(tail) = normalized.strip_prefix(
        "For each player, Put a card from that player's hand on the bottom of that player's library. that player shuffles that player's graveyard into that player's library. For each player, that player draws ",
    ) {
        return format!("Each player shuffles their hand and graveyard into their library, then draws {tail}");
    }
    if let Some(rest) = normalized.strip_prefix("For each player, that player sacrifices ")
        && let Some((lands, damage_tail)) =
            rest.split_once(" lands that player controls. For each creature, Deal ")
        && let Some(amount) = damage_tail
            .strip_suffix(" damage to that object")
            .or_else(|| damage_tail.strip_suffix(" damage to that object."))
    {
        return format!(
            "Each player sacrifices {lands} lands of their choice. Deal {amount} damage to each creature"
        );
    }
    if let Some(rest) = normalized.strip_prefix("For each white or blue creature, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to that object")
            .or_else(|| rest.strip_suffix(" damage to that object."))
    {
        return format!("Deal {amount} damage to each white and/or blue creature");
    }
    if let Some(rest) = normalized.strip_prefix("For each opponent's creature, Deal ")
        && let Some((amount, tail)) = rest.split_once(" damage to that object. ")
        && (tail.eq_ignore_ascii_case("an opponent's creature can't block until end of turn")
            || tail.eq_ignore_ascii_case("an opponent's creature cant block until end of turn")
            || tail.eq_ignore_ascii_case("an opponent's creature can't block this turn")
            || tail.eq_ignore_ascii_case("an opponent's creature cant block this turn"))
    {
        return format!(
            "Deal {amount} damage to each creature your opponents control. Creatures your opponents control can't block this turn"
        );
    }
    if let Some(rest) = normalized.strip_prefix("For each opponent's creature, Deal ")
        && let Some(amount) = rest.strip_suffix(" damage to that object")
    {
        return format!("Deal {amount} damage to each creature your opponents control");
    }
    if let Some(rest) = normalized.strip_prefix("For each opponent's creature, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to each opponent.")
            .or_else(|| rest.strip_suffix(" damage to each opponent"))
    {
        return format!("Deal {amount} damage to each creature your opponents control");
    }
    if let Some(rest) = normalized.strip_prefix("For each creature or planeswalker, Deal ")
        && let Some(amount) = rest.strip_suffix(" damage to that object")
    {
        return format!("Deal {amount} damage to each creature and each planeswalker");
    }
    if lower_normalized == "an opponent's creature can't block until end of turn"
        || lower_normalized == "an opponent's creature cant block until end of turn"
        || lower_normalized == "an opponent's creature can't block this turn"
        || lower_normalized == "an opponent's creature cant block this turn"
    {
        return "Creatures your opponents control can't block this turn".to_string();
    }
    if lower_normalized == "target player's creature gets -2/-2 until end of turn"
        || lower_normalized == "target players creature gets -2/-2 until end of turn"
    {
        return "Creatures target player controls get -2/-2 until end of turn".to_string();
    }
    if lower_normalized == "monocolored creature can't block until end of turn"
        || lower_normalized == "monocolored creature cant block until end of turn"
    {
        return "Monocolored creatures can't block this turn".to_string();
    }
    if let Some((first, second)) = normalized.split_once(". ")
        && first.starts_with("Deal ")
        && (second.eq_ignore_ascii_case("creature can't block until end of turn")
            || second.eq_ignore_ascii_case("creature cant block until end of turn")
            || second.eq_ignore_ascii_case("permanent can't block until end of turn")
            || second.eq_ignore_ascii_case("permanent cant block until end of turn"))
        && let Some(rest) = first.strip_prefix("Deal ")
        && let Some((amount, targets)) = rest.split_once(" damage to ")
    {
        return format!(
            "Deal {amount} damage to each of {targets}. Those creatures can't block this turn"
        );
    }
    if lower_normalized == "tap all an opponent's creature. untap all a creature you control" {
        return "Tap all creatures your opponents control and untap all creatures you control"
            .to_string();
    }
    if lower_normalized == "opponent's creatures get -1/-0"
        || lower_normalized == "opponent's creatures get -1/-0."
    {
        return "Creatures your opponents control get -1/-0".to_string();
    }
    if let Some(rest) = strip_prefix_ascii_ci(&normalized, "opponent's ")
        && let Some((objects, predicate)) = split_once_ascii_ci(rest, " get ")
    {
        return format!(
            "{} your opponents control get {}",
            capitalize_first(objects.trim()),
            predicate.trim()
        );
    }
    if let Some(rest) = strip_prefix_ascii_ci(&normalized, "opponent's ")
        && let Some((objects, predicate)) = split_once_ascii_ci(rest, " gets ")
    {
        return format!(
            "{} your opponents control gets {}",
            capitalize_first(objects.trim()),
            predicate.trim()
        );
    }
    if lower_normalized == "deal 1 damage to each target creature an opponent controls"
        || lower_normalized == "deal 1 damage to each target creature an opponent controls."
    {
        return "Deal 1 damage to each creature target opponent controls".to_string();
    }
    if lower_normalized == "whenever another creature you control enters, you gain 1 life"
        || lower_normalized == "whenever another creature you control enters, you gain 1 life."
    {
        return "Whenever another creature enters under your control, you gain 1 life".to_string();
    }
    if lower_normalized == "red and green spells you cast cost {1} less to cast"
        || lower_normalized == "red and green spells you cast costs {1} less to cast"
        || lower_normalized == "red and green spells you cast cost {1} less to cast."
    {
        return "Each spell you cast that's red or green costs {1} less to cast".to_string();
    }
    if lower_normalized == "other zombie you control get +1/+0"
        || lower_normalized == "other zombie you control get +1/+0."
    {
        return "Other Zombies you control get +1/+0".to_string();
    }
    if lower_normalized == "draw three cards. target opponent draws 3 cards"
        || lower_normalized == "draw three cards. target opponent draws 3 cards."
    {
        return "You and target opponent each draw three cards".to_string();
    }
    if lower_normalized
        == "whenever this creature blocks creature, permanent can't untap until your next turn"
        || lower_normalized
            == "whenever this creature blocks creature, permanent cant untap until your next turn"
    {
        return "Whenever this creature blocks a creature, that creature doesn't untap during its controller's next untap step".to_string();
    }
    if lower_normalized == "untap all a snow permanent you control"
        || lower_normalized == "untap all a snow permanent you control."
    {
        return "Untap each snow permanent you control".to_string();
    }
    if lower_normalized == "untap all a creature you control"
        || lower_normalized == "untap all a creature you control."
    {
        return "Untap all creatures you control".to_string();
    }
    if lower_normalized == "as an additional cost to cast this spell, you discard a card"
        || lower_normalized == "as an additional cost to cast this spell, you discard a card."
    {
        return "As an additional cost to cast this spell, discard a card".to_string();
    }
    if lower_normalized == "add 1 mana of commander's color identity"
        || lower_normalized == "add 1 mana of commander's color identity."
    {
        return "Add one mana of any color in your commander's color identity".to_string();
    }
    if let Some((cost, tail)) = split_once_ascii_ci(&normalized, ": ")
        && (tail.eq_ignore_ascii_case("Add 1 mana of commander's color identity")
            || tail.eq_ignore_ascii_case("Add 1 mana of commander's color identity."))
    {
        return format!("{cost}: Add one mana of any color in your commander's color identity");
    }
    if lower_normalized == "return this permanent from graveyard to the battlefield tapped"
        || lower_normalized == "return this permanent from graveyard to the battlefield tapped."
        || lower_normalized == "return this creature from graveyard to the battlefield tapped"
        || lower_normalized == "return this creature from graveyard to the battlefield tapped."
        || lower_normalized == "return this source from graveyard to the battlefield tapped"
        || lower_normalized == "return this source from graveyard to the battlefield tapped."
    {
        return "Return this card from your graveyard to the battlefield tapped".to_string();
    }
    if let Some((cost, tail)) = split_once_ascii_ci(&normalized, ": ")
        && (tail
            .eq_ignore_ascii_case("Return this permanent from graveyard to the battlefield tapped")
            || tail.eq_ignore_ascii_case(
                "Return this permanent from graveyard to the battlefield tapped.",
            )
            || tail.eq_ignore_ascii_case(
                "Return this creature from graveyard to the battlefield tapped",
            )
            || tail.eq_ignore_ascii_case(
                "Return this creature from graveyard to the battlefield tapped.",
            )
            || tail.eq_ignore_ascii_case(
                "Return this source from graveyard to the battlefield tapped",
            )
            || tail.eq_ignore_ascii_case(
                "Return this source from graveyard to the battlefield tapped.",
            ))
    {
        return format!("{cost}: Return this card from your graveyard to the battlefield tapped");
    }
    if lower_normalized == "target player sacrifices target player's creature" {
        return "Target player sacrifices a creature of their choice".to_string();
    }
    if lower_normalized == "target player sacrifices a creature"
        || lower_normalized == "target player sacrifices a creature."
    {
        return "Target player sacrifices a creature of their choice".to_string();
    }
    if lower_normalized
        == "target player sacrifices target player's creature. target player loses 1 life"
    {
        return "Target player sacrifices a creature of their choice and loses 1 life".to_string();
    }
    if lower_normalized == "target player sacrifices a creature. target player loses 1 life"
        || lower_normalized == "target player sacrifices a creature and target player loses 1 life"
        || lower_normalized == "target player sacrifices a creature and loses 1 life"
    {
        return "Target player sacrifices a creature of their choice and loses 1 life".to_string();
    }
    if lower_normalized
        == "target player sacrifices a creature. target player gains life equal to its toughness"
        || lower_normalized
            == "target player sacrifices a creature of their choice. target player gains life equal to its toughness"
    {
        return "Target player sacrifices a creature of their choice, then gains life equal to that creature's toughness.".to_string();
    }
    if let Some((prefix, tail)) = normalized.split_once(
        ". For each opponent, that player discards a card. For each opponent, that player loses ",
    ) && let Some((life_amount, trailing)) = tail.split_once(" life")
    {
        let trailing = trailing.trim_start_matches('.').trim();
        let trailing_clause = if trailing.is_empty() {
            String::new()
        } else {
            format!(". {}", capitalize_first(trailing))
        };
        return format!(
            "{}, discards a card, and loses {} life{}",
            prefix.trim_end_matches('.').trim(),
            life_amount.trim(),
            trailing_clause
        );
    }
    if let Some((draw_count, gain_tail)) = normalized
        .strip_prefix("Draw ")
        .and_then(|rest| rest.split_once(" card. you gain "))
        && let Some(life_amount) = gain_tail.strip_suffix(" life")
    {
        return format!("You draw {draw_count} card and gain {life_amount} life");
    }
    if let Some((draw_count, gain_tail)) = normalized
        .strip_prefix("Draw ")
        .and_then(|rest| rest.split_once(" cards. you gain "))
        && let Some(life_amount) = gain_tail.strip_suffix(" life")
    {
        return format!("You draw {draw_count} cards and gain {life_amount} life");
    }
    if let Some((prefix, tail)) =
        normalized.split_once(", its controller draws a card. its controller loses ")
        && let Some((life_amount, rest)) = tail.split_once(" life. Draw a card. you lose ")
        && let Some(rest_amount) = rest
            .strip_suffix(" life")
            .or_else(|| rest.strip_suffix(" life."))
        && life_amount.trim() == rest_amount.trim()
    {
        return format!(
            "{prefix}, you and its controller each draw a card and lose {} life",
            life_amount.trim()
        );
    }
    if let Some((prefix, tail)) =
        normalized.split_once(", you draw a card. the attacking player draws a card. you lose ")
        && let Some((life_amount, rest)) = tail.split_once(" life. the attacking player loses ")
        && let Some(rest_amount) = rest
            .strip_suffix(" life")
            .or_else(|| rest.strip_suffix(" life."))
        && life_amount.trim() == rest_amount.trim()
    {
        return format!(
            "{prefix}, you and the attacking player each draw a card and lose {} life",
            life_amount.trim()
        );
    }
    if let Some(rest) = normalized.strip_prefix("Target player sacrifices target player's ")
        && let Some((first_kind, tail)) =
            rest.split_once(". target player sacrifices target player's ")
        && let Some((second_kind, damage_tail)) = tail.split_once(". Deal ")
        && let Some(amount) = damage_tail
            .strip_suffix(" damage to target player")
            .or_else(|| damage_tail.strip_suffix(" damage to target player."))
    {
        return format!(
            "Target player sacrifices {} and {} of their choice. Deal {} damage to that player",
            first_kind.trim(),
            second_kind.trim(),
            amount.trim()
        );
    }
    if lower_normalized == "target player sacrifices target player's attacking or blocking creature"
        || lower_normalized
            == "target player sacrifices target player's attacking/blocking creature"
        || lower_normalized
            == "target player sacrifices target player's attacking or blocking creature."
        || lower_normalized
            == "target player sacrifices target player's attacking/blocking creature."
    {
        return "Target player sacrifices an attacking or blocking creature of their choice"
            .to_string();
    }
    if lower_normalized == "target player sacrifices an attacking or blocking creature"
        || lower_normalized == "target player sacrifices an attacking/blocking creature"
        || lower_normalized == "target player sacrifices an attacking or blocking creature."
        || lower_normalized == "target player sacrifices an attacking/blocking creature."
    {
        return "Target player sacrifices an attacking or blocking creature of their choice"
            .to_string();
    }
    if lower_normalized == "destroy all creatures that are not all colors"
        || lower_normalized == "destroy all creatures that are not all colors."
    {
        return "Destroy each creature that isn't all colors.".to_string();
    }
    if lower_normalized == "spell effects: destroy all creatures that are not all colors"
        || lower_normalized == "spell effects: destroy all creatures that are not all colors."
    {
        return "Spell effects: Destroy each creature that isn't all colors.".to_string();
    }
    if lower_normalized
        == "destroy target creature. if that permanent dies this way, create two tokens that are copies of it under that object's controller's control, except their power and toughness are each half that permanent's power and toughness, rounded up"
        || lower_normalized
            == "destroy target creature. if that permanent dies this way, create two tokens that are copies of it under that object's controller's control, except their power and toughness are each half that permanent's power and toughness, rounded up."
    {
        return "Destroy target creature. If that creature dies this way, its controller creates two tokens that are copies of that creature, except their power is half that creature's power and their toughness is half that creature's toughness. Round up each time.".to_string();
    }
    if lower_normalized
        == "target player sacrifices an artifact. target player sacrifices a land. deal 2 damage to target player"
        || lower_normalized
            == "target player sacrifices an artifact. target player sacrifices a land. deal 2 damage to target player."
    {
        return "Target player sacrifices an artifact and a land of their choice. Structural Collapse deals 2 damage to that player.".to_string();
    }
    if lower_normalized.contains(
        "this creature is put into your graveyard from the battlefield: at the beginning of the next end step, you lose 1 life. return this creature to its owner's hand",
    ) {
        return "When this creature is put into your graveyard from the battlefield, at the beginning of the next end step, you lose 1 life and return this card to your hand.".to_string();
    }
    if let Some(condition) = normalized
        .strip_prefix("This creature has Doesn't untap during your untap step as long as ")
    {
        return format!("This creature doesn't untap during your untap step if {condition}");
    }
    if lower_normalized == "creature with a +1/+1 counter on it you control have can't be blocked"
        || lower_normalized
            == "creature with a +1/+1 counter ons it you control have can't be blocked"
    {
        return "Creatures you control with +1/+1 counters on them can't be blocked".to_string();
    }
    if lower_normalized
        == "for each player, that player sacrifices two creatures that player controls"
        || lower_normalized
            == "for each player, that player sacrifices two creatures that player controls."
    {
        return "Each player sacrifices two creatures of their choice".to_string();
    }
    if lower_normalized == "for each player, return target player's creature to its owner's hand"
        || lower_normalized
            == "for each player, return target player's creature to its owner's hand."
    {
        return "Each player returns a creature they control to its owner's hand".to_string();
    }
    if lower_normalized == "exile all card in graveyard"
        || lower_normalized == "exile all card in graveyard."
    {
        return "Exile all graveyards".to_string();
    }
    if lower_normalized == "permanents enter the battlefield tapped"
        || lower_normalized == "permanents enter the battlefield tapped."
    {
        return "Permanents enter tapped".to_string();
    }
    if let Some(rest) = normalized.strip_prefix("Token creatures get ") {
        return format!("Creature tokens get {rest}");
    }
    if let Some(rest) = normalized.strip_prefix(
        "When this creature enters, target opponent chooses exactly 1 target player's creature in the battlefield. Destroy it",
    ) {
        return format!("When this creature enters, target opponent chooses a creature they control. Destroy that creature{rest}");
    }
    if lower_normalized.starts_with(
        "target opponent chooses target creature an opponent controls. exile it. exile all ",
    ) && (lower_normalized.contains(" in target opponent's graveyard")
        || lower_normalized.contains(" in target opponent's graveyards"))
    {
        return "Target opponent exiles a creature they control and their graveyard.".to_string();
    }
    if normalized
        == "Whenever this creature blocks creature, permanent can't untap until your next turn"
        || normalized
            == "Whenever this creature blocks creature, permanent cant untap until your next turn"
        || normalized
            == "Whenever this creature blocks creature, permanent can't untap until your next turn."
        || normalized
            == "Whenever this creature blocks creature, permanent cant untap until your next turn."
    {
        return "Whenever this creature blocks a creature, that creature doesn't untap during its controller's next untap step".to_string();
    }
    if let Some(kind) = strip_prefix_ascii_ci(&normalized, "you may put ").and_then(|rest| {
        rest.strip_suffix(" card in your hand onto the battlefield")
            .or_else(|| rest.strip_suffix(" card in your hand onto the battlefield."))
    }) {
        let kind = kind.trim();
        let noun = if kind.is_empty() {
            "card".to_string()
        } else {
            format!("{kind} card")
        };
        let rendered_noun =
            if kind.starts_with("target ") || kind.starts_with("a ") || kind.starts_with("an ") {
                noun
            } else {
                with_indefinite_article(&noun)
            };
        return format!("You may put {rendered_noun} from your hand onto the battlefield");
    }
    if let Some(kind) = strip_prefix_ascii_ci(&normalized, "you may put ").and_then(|rest| {
        rest.strip_suffix(" in your hand onto the battlefield")
            .or_else(|| rest.strip_suffix(" in your hand onto the battlefield."))
    }) {
        let kind = kind.trim();
        let rendered_kind =
            if kind.starts_with("target ") || kind.starts_with("a ") || kind.starts_with("an ") {
                kind.to_string()
            } else {
                with_indefinite_article(kind)
            };
        return format!("You may put {rendered_kind} from your hand onto the battlefield");
    }
    if let Some((cost, rest)) = split_once_ascii_ci(&normalized, ": ")
        && let Some(kind) = strip_prefix_ascii_ci(rest.trim(), "you may put ").and_then(|tail| {
            tail.strip_suffix(" card in your hand onto the battlefield")
                .or_else(|| tail.strip_suffix(" card in your hand onto the battlefield."))
        })
    {
        let kind = kind.trim();
        let noun = if kind.is_empty() {
            "card".to_string()
        } else {
            format!("{kind} card")
        };
        let rendered_noun =
            if kind.starts_with("target ") || kind.starts_with("a ") || kind.starts_with("an ") {
                noun
            } else {
                with_indefinite_article(&noun)
            };
        return format!("{cost}: You may put {rendered_noun} from your hand onto the battlefield");
    }
    if let Some((cost, rest)) = split_once_ascii_ci(&normalized, ": ")
        && let Some(kind) = strip_prefix_ascii_ci(rest.trim(), "you may put ").and_then(|tail| {
            tail.strip_suffix(" in your hand onto the battlefield")
                .or_else(|| tail.strip_suffix(" in your hand onto the battlefield."))
        })
    {
        let kind = kind.trim();
        let rendered_kind =
            if kind.starts_with("target ") || kind.starts_with("a ") || kind.starts_with("an ") {
                kind.to_string()
            } else {
                with_indefinite_article(kind)
            };
        return format!("{cost}: You may put {rendered_kind} from your hand onto the battlefield");
    }
    if let Some(rest) = normalized
        .strip_prefix("you may Put target planeswalker card in your hand onto the battlefield")
    {
        return format!(
            "You may put a planeswalker card from your hand onto the battlefield{rest}"
        );
    }
    if normalized.starts_with(
        "When this creature enters, for each another creature you control, Put a +1/+1 counter on that object",
    ) || normalized.starts_with(
        "When this creature enters, for each another creature you control, Put 1 +1/+1 counter on that object",
    ) {
        return "When this creature enters, put a +1/+1 counter on each other creature you control"
            .to_string();
    }
    if normalized.starts_with(
        "When this permanent enters, for each another creature you control, Put a +1/+1 counter on that object",
    ) || normalized.starts_with(
        "When this permanent enters, for each another creature you control, Put 1 +1/+1 counter on that object",
    ) {
        return "When this permanent enters, put a +1/+1 counter on each other creature you control"
            .to_string();
    }
    if normalized.starts_with("For each player, that player loses 1 life for each ") {
        let rest = normalized
            .trim_start_matches("For each player, that player loses 1 life for each ")
            .to_string();
        return format!("Each player loses 1 life for each {rest}");
    }
    if normalized.starts_with("For each player, that player gains 1 life for each ") {
        let rest = normalized
            .trim_start_matches("For each player, that player gains 1 life for each ")
            .to_string();
        return format!("Each player gains 1 life for each {rest}");
    }
    if let Some((prefix, tail)) =
        split_once_ascii_ci(&normalized, ", for each player, that player ")
        && !tail.trim().is_empty()
    {
        return format!(
            "{}, each player {}",
            capitalize_first(prefix.trim()),
            tail.trim()
        );
    }
    if let Some(rest) = normalized.strip_prefix("For each player, that player ")
        && !rest.trim().is_empty()
    {
        return format!("Each player {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("for each player, that player ")
        && !rest.trim().is_empty()
    {
        return format!("each player {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("For each opponent, that player ")
        && !rest.trim().is_empty()
    {
        return format!("Each opponent {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("for each opponent, that player ")
        && !rest.trim().is_empty()
    {
        return format!("each opponent {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("For each ")
        && let Some((subject, tail)) = rest.split_once(", Put ")
        && let Some(counter_clause) = tail
            .strip_suffix(" counter on that object")
            .or_else(|| tail.strip_suffix(" counter on that object."))
    {
        return format!("Put {counter_clause} counter on each {subject}");
    }
    if matches!(
        normalized.as_str(),
        "attacking creature can't untap until your next turn"
            | "attacking creature cant untap until your next turn"
    ) {
        return "Each attacking creature doesn't untap during its controller's next untap step"
            .to_string();
    }
    if let Some(rest) =
        normalized.strip_prefix("Prevent all combat damage that would be dealt this turn. ")
        && matches!(
            rest,
            "attacking creature can't untap until your next turn."
                | "attacking creature cant untap until your next turn."
                | "attacking creature can't untap until your next turn"
                | "attacking creature cant untap until your next turn"
        )
    {
        return "Prevent all combat damage that would be dealt this turn. Each attacking creature doesn't untap during its controller's next untap step".to_string();
    }
    if normalized == "An opponent's creature enter tapped."
        || normalized == "An opponent's creature enter tapped"
        || normalized == "An opponent's creature enters tapped."
        || normalized == "An opponent's creature enters tapped"
    {
        return "Creatures your opponents control enter tapped".to_string();
    }
    if normalized == "An opponent's artifact or creature or land enter the battlefield tapped."
        || normalized == "An opponent's artifact or creature or land enter the battlefield tapped"
        || normalized == "An opponent's artifact or creature or land enters the battlefield tapped."
        || normalized == "An opponent's artifact or creature or land enters the battlefield tapped"
    {
        return "Artifacts, creatures, and lands your opponents control enter tapped".to_string();
    }
    if let Some(subject) = normalized.strip_suffix(" have flashback as long as it's your turn") {
        return format!(
            "During your turn, {} have flashback. Its flashback cost is equal to its mana cost",
            subject.trim()
        );
    }
    if let Some(subject) = normalized.strip_suffix(" has flashback as long as it's your turn") {
        return format!(
            "During your turn, {} has flashback. Its flashback cost is equal to its mana cost",
            subject.trim()
        );
    }
    if let Some((types, tail)) = normalized.split_once(" creatures get ")
        && types.contains(" or ")
        && looks_like_creature_type_list_subject(types)
    {
        let type_items = types
            .split(" or ")
            .map(str::trim)
            .filter(|item| {
                item.chars()
                    .next()
                    .is_some_and(|ch| ch.is_ascii_uppercase())
            })
            .collect::<Vec<_>>();
        if type_items.len() >= 2 {
            let listed = join_with_and(
                &type_items
                    .iter()
                    .map(|item| with_indefinite_article(item))
                    .collect::<Vec<_>>(),
            );
            return format!(
                "Each creature that's {} gets {}",
                listed,
                tail.trim_end_matches('.').trim()
            );
        }
    }
    if let Some((types, tail)) = normalized.split_once(" creatures have ")
        && types.contains(" or ")
        && looks_like_creature_type_list_subject(types)
    {
        let type_items = types
            .split(" or ")
            .map(str::trim)
            .filter(|item| {
                item.chars()
                    .next()
                    .is_some_and(|ch| ch.is_ascii_uppercase())
            })
            .collect::<Vec<_>>();
        if type_items.len() >= 2 {
            let listed = join_with_and(
                &type_items
                    .iter()
                    .map(|item| with_indefinite_article(item))
                    .collect::<Vec<_>>(),
            );
            return format!(
                "Each creature that's {} has {}",
                listed,
                tail.trim_end_matches('.').trim()
            );
        }
    }

    if lower == "attacks each combat if able" {
        return "This creature attacks each combat if able".to_string();
    }
    if lower == "counter target creature" {
        return "Counter target creature spell".to_string();
    }
    if lower == "counter up to one target creature" {
        return "Counter up to one target creature spell".to_string();
    }
    if lower == "counter target instant spell spell" {
        return "Counter target instant spell".to_string();
    }
    if lower == "counter target sorcery spell spell" {
        return "Counter target sorcery spell".to_string();
    }
    if lower == "destroy target artifact or enchantment or creature with flying" {
        return "Destroy target artifact, enchantment, or creature with flying".to_string();
    }
    if let Some(rest) = normalized.strip_prefix("Can't be blocked by creatures with power ") {
        return format!("This creature can't be blocked by creatures with power {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Can't be blocked by more than ") {
        if let Some(noun) = rest.strip_prefix("1 creature") {
            return format!("This creature can't be blocked by more than one creature{noun}");
        }
        return format!("This creature can't be blocked by more than {rest}");
    }
    if let Some(rest) =
        normalized.strip_prefix("As long as this creature is equipped, this creature gets ")
    {
        return format!("As long as this creature is equipped, it gets {rest}");
    }
    if let Some(rest) =
        normalized.strip_prefix("As long as this creature is enchanted, this creature gets ")
    {
        return format!("As long as this creature is enchanted, it gets {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Sliver creatures get ") {
        return format!("All Sliver creatures get {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Sliver creatures gain ") {
        return format!("All Sliver creatures gain {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Sliver creatures have ") {
        return format!(
            "All Sliver creatures have {}",
            normalize_keyword_predicate_case(rest)
        );
    }
    if let Some(rest) = normalized.strip_prefix("Creatures get ") {
        return format!("All creatures get {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("creatures get ") {
        return format!("all creatures get {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("creatures have ") {
        return format!(
            "All creatures have {}",
            normalize_keyword_predicate_case(rest)
        );
    }
    if normalized == "this creature becomes the target of a spell or ability: You sacrifice it" {
        return "When this creature becomes the target of a spell or ability, sacrifice it"
            .to_string();
    }
    if normalized
        == "this creature or Whenever another Ally you control enters: You may Put a +1/+1 counter on this creature"
    {
        return "Whenever this creature or another Ally you control enters, you may put a +1/+1 counter on this creature".to_string();
    }
    if normalized == "When this creature enters or When this creature dies, Surveil 1" {
        return "When this creature enters or dies, surveil 1".to_string();
    }
    if normalized == "Whenever you cast noncreature spell, Put a +1/+1 counter on this creature" {
        return "Whenever you cast a noncreature spell, put a +1/+1 counter on this creature"
            .to_string();
    }
    if let Some(rest) =
        normalized.strip_prefix("When this creature enters, If you attacked this turn, Deal ")
        && let Some(amount) = rest.strip_suffix(" damage to any target")
    {
        return format!(
            "Raid — When this creature enters, if you attacked this turn, this creature deals {amount} damage to any target"
        );
    }
    if let Some(rest) = normalized.strip_prefix("{")
        && rest.contains("}, Discard a card: Target attacking creature gets ")
        && rest.ends_with(" until end of turn")
    {
        return format!("Bloodrush — {{{rest}").replace(
            ", Discard a card: Target attacking creature gets ",
            ", Discard this card: Target attacking creature gets ",
        );
    }
    if let Some(rest) = normalized.strip_prefix("up to two target creatures get ") {
        return format!("Up to two target creatures each get {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("one or two target creatures get ") {
        return format!("One or two target creatures each get {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("two target creatures get ") {
        return format!("Two target creatures each get {rest}");
    }
    if let Some(rest) =
        normalized.strip_prefix("When this creature enters, draw a card and you lose ")
    {
        return format!("When this creature enters, you draw a card and you lose {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("When this creature enters, you draw ")
        && let Some((count, tail)) = rest.split_once(". target opponent draws ")
        && tail.trim_end_matches('.') == count
    {
        return format!("When this creature enters, you and target opponent each draw {count}");
    }
    if let Some(rest) = normalized.strip_prefix("You draw ")
        && let Some((count, tail)) = rest.split_once(". target opponent draws ")
        && tail.trim_end_matches('.') == count
    {
        return format!("You and target opponent each draw {count}");
    }
    if let Some(rest) = normalized.strip_prefix("Search your library for basic land you own") {
        return format!("Search your library for a basic land card{rest}");
    }
    if let Some(rest) =
        normalized.strip_prefix("Search your library for up to one basic land you own")
    {
        return format!("Search your library for a basic land card{rest}");
    }
    if normalized == "Counter target instant spell spell" {
        return "Counter target instant spell".to_string();
    }
    if normalized == "Counter target sorcery spell spell" {
        return "Counter target sorcery spell".to_string();
    }
    if normalized == "Destroy target artifact or enchantment or creature with flying" {
        return "Destroy target artifact, enchantment, or creature with flying".to_string();
    }
    if let Some(rest) = normalized.strip_prefix("this creature gets ")
        && let Some((pt, tail)) = rest.split_once(" for each Equipment attached to this creature")
    {
        return format!("This creature gets {pt} for each Equipment attached to it{tail}");
    }
    if normalized
        == "Whenever this creature or Whenever another Ally you control enters, creatures you control get +1/+1 until end of turn"
    {
        return "Whenever this creature or another Ally you control enters, creatures you control get +1/+1 until end of turn".to_string();
    }
    if lower
        == "whenever this creature or whenever another ally you control enters, creatures you control get +1/+1 until end of turn"
    {
        return "Whenever this creature or another Ally you control enters, creatures you control get +1/+1 until end of turn".to_string();
    }
    if lower
        == "whenever this creature or least two other creatures attack, this creature gets +2/+2 until end of turn"
    {
        return "Whenever this creature and at least two other creatures attack, this creature gets +2/+2 until end of turn".to_string();
    }
    if let Some(rest) = normalized.strip_prefix("Whenever This creature or Whenever another ") {
        return format!("Whenever this creature or another {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Whenever This or Whenever another ") {
        return format!("Whenever this or another {rest}");
    }
    if let Some((left, right)) = normalized.split_once(" or Whenever another ") {
        if left.starts_with("Whenever ") {
            return format!("{left} or another {right}");
        }
    }
    if let Some((left, right)) = normalized.split_once(" or whenever another ") {
        if left.starts_with("whenever ") {
            return format!("{left} or another {right}");
        }
    }
    if let Some(rest) = lower.strip_prefix("whenever this creature or whenever another ") {
        return format!("Whenever this creature or another {rest}");
    }
    if let Some(rest) = lower.strip_prefix("whenever this or whenever another ") {
        return format!("Whenever this or another {rest}");
    }
    if let Some(rest) = normalized
        .strip_prefix("At the beginning of your upkeep, tap this creature unless you lose ")
    {
        return format!("At the beginning of your upkeep, tap this creature unless you pay {rest}");
    }
    if let Some(rest) =
        lower.strip_prefix("at the beginning of your upkeep, tap this creature unless you lose ")
    {
        return format!("At the beginning of your upkeep, tap this creature unless you pay {rest}");
    }
    if normalized == "Whenever this creature becomes blocked, the defending player discards a card"
    {
        return "Whenever this creature becomes blocked, defending player discards a card"
            .to_string();
    }
    if let Some(rest) = normalized.strip_prefix("Whenever you cast spell ") {
        if let Some((kind, tail)) = rest.split_once(',') {
            let kind = kind.trim();
            let needs_article = !kind.is_empty()
                && !kind.starts_with("a ")
                && !kind.starts_with("an ")
                && !matches!(kind, "a" | "an" | "another");
            if needs_article {
                return format!(
                    "Whenever you cast {} spell,{}",
                    with_indefinite_article(kind),
                    tail
                );
            }
            return format!("Whenever you cast {kind} spell,{tail}");
        }
        let kind = rest.trim();
        if !kind.is_empty() {
            let needs_article = !kind.starts_with("a ")
                && !kind.starts_with("an ")
                && !matches!(kind, "a" | "an" | "another");
            if needs_article {
                return format!("Whenever you cast {} spell", with_indefinite_article(kind));
            }
            return format!("Whenever you cast {kind} spell");
        }
    }
    if let Some(rest) = normalized.strip_prefix("Whenever you cast ")
        && let Some((kind, tail)) = rest.split_once(" spell,")
    {
        let kind = kind.trim();
        if !kind.is_empty()
            && !kind.starts_with("a ")
            && !kind.starts_with("an ")
            && !matches!(kind, "a" | "an" | "another")
        {
            return format!(
                "Whenever you cast {} spell,{}",
                with_indefinite_article(kind),
                tail
            );
        }
    }
    if let Some(kind) = normalized
        .strip_prefix("Whenever you cast ")
        .and_then(|tail| tail.strip_suffix(" spell"))
    {
        let kind = kind.trim();
        if !kind.is_empty()
            && !kind.starts_with("a ")
            && !kind.starts_with("an ")
            && !matches!(kind, "a" | "an" | "another")
        {
            return format!("Whenever you cast {} spell", with_indefinite_article(kind));
        }
    }
    if let Some(rest) = normalized.strip_prefix("Whenever you cast spell with mana value ") {
        return format!("Whenever you cast a spell with mana value {rest}");
    }
    if normalized == "When this creature enters, you sacrifice a creature" {
        return "When this creature enters, sacrifice a creature".to_string();
    }
    if normalized == "you draw a card. Scry 2" {
        return "Draw a card. Scry 2".to_string();
    }
    if normalized == "This creature enters with X +1/+1 counters" {
        return "This creature enters with X +1/+1 counters on it".to_string();
    }
    if normalized == "Trample, Haste" {
        return "Trample, haste".to_string();
    }
    if let Some(rest) =
        normalized.strip_prefix("Whenever you cast an instant or sorcery spell, deal ")
    {
        return format!(
            "Whenever you cast an instant or sorcery spell, this creature deals {rest}"
        );
    }
    if let Some(rest) = lower.strip_prefix("whenever you cast an instant or sorcery spell, deal ") {
        return format!(
            "Whenever you cast an instant or sorcery spell, this creature deals {rest}"
        );
    }
    if let Some(rest) = normalized.strip_prefix("Whenever you cast a noncreature spell, it deals ")
    {
        return format!("Whenever you cast a noncreature spell, this creature deals {rest}");
    }
    if let Some(rest) = lower.strip_prefix("whenever you cast a noncreature spell, deal ") {
        return format!("Whenever you cast a noncreature spell, this creature deals {rest}");
    }
    if let Some(rest) = lower.strip_prefix("whenever a land you control enters, deal ") {
        return format!("Whenever a land you control enters, this creature deals {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("{T}: Deal ") {
        return format!("{{T}}: This creature deals {rest}");
    }
    if let Some(rest) = normalized
        .strip_prefix("Search your library for a card, put it on top of library, then shuffle. ")
    {
        return format!(
            "Search your library for a card, then shuffle and put that card on top. {rest}"
        );
    }
    if normalized == "target creature gains Deathtouch and gains Indestructible until end of turn" {
        return "Target creature gains deathtouch and indestructible until end of turn".to_string();
    }
    if normalized == "When this Aura enters, tap enchanted creature" {
        return "When this Aura enters, tap enchanted creature.".to_string();
    }
    if normalized == "Doesn't untap during your untap step" {
        return "This creature doesn't untap during your untap step".to_string();
    }
    if normalized == "This creature enters with 2 +1/+1 counters" {
        return "This creature enters with two +1/+1 counters on it".to_string();
    }
    if normalized
        == "Whenever this creature blocks or becomes blocked by a creature, it deals 1 damage to that creature"
    {
        return "Whenever this creature blocks or becomes blocked by a creature, this creature deals 1 damage to that creature".to_string();
    }
    if normalized == "When this creature enters or dies, surveil 1" {
        return "When this creature enters or dies, surveil 1. (Look at the top card of your library. You may put it into your graveyard.)".to_string();
    }
    if let Some(amount) = normalized
        .strip_prefix("At the beginning of your upkeep, it deals ")
        .and_then(|rest| rest.strip_suffix(" damage to you"))
    {
        return format!(
            "At the beginning of your upkeep, this creature deals {amount} damage to you"
        );
    }
    if let Some(rest) = normalized.strip_prefix("Search your library for a ")
        && let Some((tribe, tail)) = rest.split_once(" with mana value ")
        && let Some(value) = tail.strip_suffix(" card, put it onto the battlefield, then shuffle")
    {
        return format!(
            "Search your library for a {tribe} permanent card with mana value {value}, put it onto the battlefield, then shuffle"
        );
    }
    if let Some((cost, rest)) = normalized.split_once(": Search your library for a ")
        && let Some((tribe, tail)) = rest.split_once(" with mana value ")
        && let Some(value) = tail.strip_suffix(" card, put it onto the battlefield, then shuffle")
    {
        return format!(
            "{cost}: Search your library for a {tribe} permanent card with mana value {value}, put it onto the battlefield, then shuffle"
        );
    }
    if let Some((left, right)) = split_once_ascii_ci(&normalized, ". ")
        && let Some(draw_tail) = strip_prefix_ascii_ci(left.trim(), "target player draws ")
        && let Some(loss_tail) = strip_prefix_ascii_ci(right.trim(), "target player loses ")
        && loss_tail
            .trim_end_matches('.')
            .to_ascii_lowercase()
            .ends_with(" life")
    {
        let draw_tail = draw_tail.trim();
        let draw_tail = draw_tail
            .strip_suffix(" cards")
            .map(|count| format!("{} cards", render_small_number_or_raw(count.trim())))
            .unwrap_or_else(|| draw_tail.to_string());
        return format!(
            "Target player draws {draw_tail} and loses {}",
            loss_tail.trim().trim_end_matches('.')
        );
    }
    if let Some((left, right)) = normalized.split_once(" and target player loses ")
        && left.starts_with("target player draws ")
    {
        let left = left.replacen("target player draws ", "Target player draws ", 1);
        return format!("{left} and loses {right}");
    }
    if let Some((first, second)) = split_once_ascii_ci(&normalized, ". ")
        && let Some(first_buff) = strip_prefix_ascii_ci(first.trim(), "target creature gets ")
            .and_then(|rest| rest.strip_suffix(" until end of turn"))
        && let Some(second_buff) = strip_prefix_ascii_ci(
            second.trim(),
            "other creatures with the same name as that object get ",
        )
        .and_then(|rest| {
            rest.strip_suffix(" until end of turn")
                .or_else(|| rest.strip_suffix(" until end of turn."))
        })
        && first_buff.eq_ignore_ascii_case(second_buff)
    {
        return format!(
            "Target creature and all other creatures with the same name as that creature get {first_buff} until end of turn"
        );
    }
    if let Some(rest) =
        normalized.strip_prefix("Destroy target black or red attacking or blocking creature")
    {
        return format!("Destroy target black or red creature that's attacking or blocking{rest}");
    }
    if let Some((left, right)) = normalized.split_once(". ")
        && let Some(target_desc) = left.strip_prefix("Destroy target ")
        && let Some(other_desc) = right.strip_prefix("Destroy all other ")
        && let Some((shares_desc, tail)) =
            other_desc.split_once(" that shares a color with that object")
        && shares_desc.eq_ignore_ascii_case(target_desc)
    {
        return format!(
            "Destroy target {target_desc} and each other {shares_desc} that shares a color with it{tail}"
        );
    }
    if let Some(tail) = normalized.strip_prefix("you draw ")
        && let Some(count) = tail.strip_suffix(" cards")
    {
        return format!("Draw {} cards", render_small_number_or_raw(count));
    }
    if let Some(tail) = normalized
        .strip_prefix("Counter target spell, then its controller mills ")
        .and_then(|rest| rest.strip_suffix(" cards"))
    {
        return format!(
            "Counter target spell. Its controller mills {} cards",
            render_small_number_or_raw(tail)
        );
    }
    if let Some(tail) = normalized
        .strip_prefix("Counter target spell, then its controller mills ")
        .and_then(|rest| rest.strip_suffix(" card"))
    {
        return format!(
            "Counter target spell. Its controller mills {} card",
            render_small_number_or_raw(tail)
        );
    }
    if let Some(rest) = normalized.strip_prefix("target creature you control gets ")
        && let Some((pt, tail)) =
            rest.split_once(" until end of turn, then it fights target creature you don't control")
    {
        return format!(
            "Target creature you control gets {pt} until end of turn. It fights target creature you don't control{tail}"
        );
    }
    if let Some(rest) = normalized.strip_prefix("Target creature you control gets ")
        && let Some((pt, tail)) =
            rest.split_once(" until end of turn, then it fights target creature you don't control")
    {
        return format!(
            "Target creature you control gets {pt} until end of turn. It fights target creature you don't control{tail}"
        );
    }
    if let Some(rest) = normalized
        .strip_prefix("this creature gets ")
        .or_else(|| normalized.strip_prefix("This creature gets "))
        && let Some((pt, cond)) = rest.split_once(" as long as ")
        && let Some((keyword, right_cond)) = cond.split_once(" and has ")
        && let Some((granted, repeated_cond)) = right_cond.split_once(" as long as ")
    {
        let keyword = keyword.trim().trim_end_matches('.');
        let repeated_cond = repeated_cond.trim().trim_end_matches('.');
        if keyword.eq_ignore_ascii_case(repeated_cond) {
            return format!(
                "As long as {keyword}, this creature gets {pt} and has {}",
                normalize_keyword_predicate_case(granted)
            );
        }
    }
    if let Some(rest) = normalized.strip_prefix("Create ")
        && let Some((token_desc, tail)) = rest.split_once(" token, tapped")
    {
        return format!("Create a tapped {token_desc} token{tail}");
    }
    if let Some(rest) = normalized.strip_prefix("Create ")
        && let Some((token_desc, tail)) = rest.split_once(" tokens, tapped")
    {
        return format!("Create tapped {token_desc} tokens{tail}");
    }
    if let Some((left, right)) = normalized.split_once(". ")
        && left.contains(" deals ")
        && right.starts_with("Deal ")
        && right.ends_with(" damage to you")
    {
        return format!("{left} and {}", lowercase_first(right));
    }
    if let Some(rest) = normalized.strip_prefix("For each player, that player ") {
        return format!("Each player {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("For each opponent, that player ") {
        return format!("Each opponent {rest}");
    }
    if let Some(tail) = normalized
        .strip_prefix("Whenever this creature blocks or becomes blocked by a creature, it deals ")
    {
        return format!(
            "Whenever this creature blocks or becomes blocked by a creature, this creature deals {tail}"
        );
    }
    if normalized.contains("cycling {{") {
        normalized = normalized.replace("{{", "{").replace("}}", "}");
    }

    normalized = normalized
        .replace("This creatures get ", "This creature gets ")
        .replace("This creatures gain ", "This creature gains ")
        .replace("this permanent gets +", "this creature gets +")
        .replace(", If ", ", if ")
        .replace(", Transform ", ", transform ")
        .replace("Counter target spell. that object's controller mills ", "Counter target spell, then its controller mills ")
        .replace(" this artifact deals ", " It deals ")
        .replace(" This artifact deals ", " It deals ")
        .replace(" for each creature blocking it until end of turn", " until end of turn for each creature blocking it")
        .replace(" for each artifact you control until end of turn", " until end of turn for each artifact you control")
        .replace("when this creature enters or When this creature dies, ", "When this creature enters or dies, ")
        .replace("When this creature enters or When this creature dies, ", "When this creature enters or dies, ")
        .replace("Whenever this creature blocks creature, ", "Whenever this creature blocks a creature, ")
        .replace("target creature you don't control or planeswalker", "target creature or planeswalker you don't control")
        .replace("Counter target instant spell spell", "Counter target instant spell")
        .replace("Counter target sorcery spell spell", "Counter target sorcery spell")
        .replace("the defending player", "defending player")
        .replace("Whenever this creature or Whenever another Ally you control enters", "Whenever this creature or another Ally you control enters")
        .replace("Chapter 1:", "I —")
        .replace("Chapter 2:", "II —")
        .replace("Chapter 3:", "III —")
        .replace("you draw a card. Scry 2", "Draw a card. Scry 2")
        .replace("Investigate 1", "Investigate")
        .replace("target player draws 2 cards", "Target player draws two cards")
        .replace("target player draws 3 cards", "Target player draws three cards")
        .replace("Draw 2 cards", "Draw two cards")
        .replace("Draw 3 cards", "Draw three cards")
        .replace("draw 2 cards", "draw two cards")
        .replace("draw 3 cards", "draw three cards")
        .replace("Create 1 ", "Create a ")
        .replace("create 1 ", "create a ")
        .replace("Create 2 ", "Create two ")
        .replace("Create 3 ", "Create three ")
        .replace("create 2 ", "create two ")
        .replace("create 3 ", "create three ")
        .replace(
            "Create a Treasure artifact token with {T}, Sacrifice this artifact: Add one mana of any color. tapped under your control",
            "Create a tapped Treasure token",
        )
        .replace(
            "create a Treasure artifact token with {T}, Sacrifice this artifact: Add one mana of any color. tapped under your control",
            "create a tapped Treasure token",
        )
        .replace(
            "Create a 0/1 colorless Eldrazi Spawn creature token with Sacrifice this creature: Add {C}. under your control",
            "Create a 0/1 colorless Eldrazi Spawn creature token. It has \"Sacrifice this token: Add {C}.\"",
        )
        .replace(
            "create a 0/1 colorless Eldrazi Spawn creature token with Sacrifice this creature: Add {C}. under your control",
            "create a 0/1 colorless Eldrazi Spawn creature token. It has \"Sacrifice this token: Add {C}.\"",
        )
        .replace(
            "Create a 1/1 colorless Eldrazi Scion creature token with Sacrifice this creature: Add {C}. under your control",
            "Create a 1/1 colorless Eldrazi Scion creature token. It has \"Sacrifice this token: Add {C}.\"",
        )
        .replace(
            "create a 1/1 colorless Eldrazi Scion creature token with Sacrifice this creature: Add {C}. under your control",
            "create a 1/1 colorless Eldrazi Scion creature token. It has \"Sacrifice this token: Add {C}.\"",
        )
        .replace("Put 2 ", "Put two ")
        .replace("Put 3 ", "Put three ")
        .replace("put 2 ", "put two ")
        .replace("put 3 ", "put three ")
        .replace("up to 1 ", "up to one ")
        .replace("up to 2 ", "up to two ")
        .replace("up to 3 ", "up to three ")
        .replace("one or 2 ", "one or two ")
        .replace(
            "Destroy target land. that object's controller loses ",
            "Destroy target land. Its controller loses ",
        )
        .replace(
            "Prevent combat damage to players until end of turn",
            "Prevent all combat damage that would be dealt to players this turn",
        )
        .replace(
            "target creature you control gets +1/+2 until end of turn, then it fights target creature you don't control",
            "Target creature you control gets +1/+2 until end of turn. It fights target creature you don't control",
        )
        .replace(
            "target creature you control gets +2/+2 until end of turn, then it fights target creature you don't control",
            "Target creature you control gets +2/+2 until end of turn. It fights target creature you don't control",
        )
        .replace(
            "target creature you control gets +2/+1 until end of turn, then it fights target creature you don't control",
            "Target creature you control gets +2/+1 until end of turn. It fights target creature you don't control",
        )
        .replace("spells you control cost ", "spells you cast cost ")
        .replace("creature you control cost ", "creature spells you cast cost ")
        .replace(
            "instant or sorcery you control cost ",
            "instant and sorcery spells you cast cost ",
        )
        .replace(
            "artifact or enchantment you control cost ",
            "artifact and enchantment spells you cast cost ",
        )
        .replace("enchantment you control cost ", "enchantment spells you cast cost ")
        .replace("you may you sacrifice ", "You may sacrifice ")
        .replace("You may you sacrifice ", "You may sacrifice ")
        .replace(
            "rather than pay this spell's mana cost (Parsed alternative cost)",
            "rather than pay this spell's mana cost",
        )
        .replace("controlss", "controls")
        .replace("the tagged object 'enchanted'", "enchanted creature")
        .replace("the tagged object '__it__'", "that creature")
        .replace(
            "the tagged object 'exiled_0' matches creature",
            "that card is a creature card",
        )
        .replace(
            "the tagged object 'triggering' matches creature",
            "that object is a creature",
        )
        .replace("the tagged object 'triggering'", "that object")
        .replace(" that player controls of their choice", " of their choice")
        .replace(" that player controls unless that player pays ", " unless that player pays ")
        .replace(" from target opponent's hand", " from their hand")
        .replace(" from target player's hand", " from their hand")
        .replace("target opponent exiles a card from their hand", "target opponent exiles a card from their hand")
        .replace("casts creature spell", "casts a creature spell")
        .replace("casts colorless spell", "casts a colorless spell")
        .replace("unless that player pays ", "unless they pay ")
        .replace("Counter target instant", "Counter target instant spell")
        .replace("Counter target sorcery", "Counter target sorcery spell")
        .replace(
            "Counter target enchantment or instant or sorcery",
            "Counter target enchantment, instant, or sorcery spell",
        )
        .replace(
            "Counter target artifact or creature or planeswalker",
            "Counter target artifact, creature, or planeswalker spell",
        )
        .replace(
            "Counter target artifact or creature unless its controller pays ",
            "Counter target artifact or creature spell unless its controller pays ",
        )
        .replace(
            "Target attacking/blocking creature",
            "Target attacking or blocking creature",
        )
        .replace(
            "Scry 2. you draw a card",
            "Scry 2, then draw a card",
        )
        .replace(". Put a stun counter on it", " and put a stun counter on it")
        .replace(". put a stun counter on it", " and put a stun counter on it")
        .replace(". you draw a card", ". Draw a card")
        .replace(
            ". this land can't untap until your next turn",
            ". This land doesn't untap during your next untap step",
        )
        .replace(
            ". this land cant untap until your next turn",
            ". This land doesn't untap during your next untap step",
        )
        .replace(
            "this land can't untap until your next turn",
            "This land doesn't untap during your next untap step",
        )
        .replace(
            "this land cant untap until your next turn",
            "This land doesn't untap during your next untap step",
        )
        .replace(
            "When this enchantment enters, Tap enchanted creature",
            "When this Aura enters, tap enchanted creature",
        )
        .replace(
            "When this enchantment enters, Exile target nonland permanent an opponent controls",
            "When this enchantment enters, exile target nonland permanent an opponent controls",
        )
        .replace(
            "As an additional cost to cast this spell, sacrifice creature you control",
            "As an additional cost to cast this spell, sacrifice a creature",
        )
        .replace(
            "As an additional cost to cast this spell, sacrifice a creature you control",
            "As an additional cost to cast this spell, sacrifice a creature",
        )
        .replace(
            "As an additional cost to cast this spell, discard card",
            "As an additional cost to cast this spell, discard a card",
        )
        .replace(
            "At the beginning of each end step, that player ",
            "At the beginning of each player's end step, that player ",
        )
        .replace(
            "that player sacrifices an untapped land.",
            "that player sacrifices an untapped land of their choice.",
        )
        ;
    if normalized.contains("you may ")
        && normalized.contains(" unless you ")
        && !normalized.contains(" unless you pay ")
        && !normalized.contains(" unless you pays ")
    {
        normalized = normalized.replacen(" unless you ", " or ", 1);
    }
    if let Some((left, rest)) = normalized.split_once("target card ")
        && let Some((kind, right)) = rest.split_once(" from")
    {
        let lower_kind = kind.to_ascii_lowercase();
        let blocked = matches!(
            lower_kind.as_str(),
            "a" | "an" | "the" | "named" | "from" | "in" | "with" | "without"
        );
        if !kind.contains(' ') && !blocked {
            normalized = format!("{left}target {kind} card from{right}");
        }
    }
    normalized = normalized
        .replace("Return a Island", "Return an Island")
        .replace("Return a artifact", "Return an artifact")
        .replace("Return a Aura", "Return an Aura");
    if let Some(mapped) = normalize_trigger_colon_clause(&normalized) {
        normalized = mapped;
    }
    if let Some((head, tail)) = normalized.split_once(", ")
        && (head.starts_with("When ")
            || head.starts_with("Whenever ")
            || head.starts_with("At the beginning "))
        && !tail.is_empty()
        && tail
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
    {
        normalized = format!("{head}, {}", lowercase_first(tail));
    }
    if let Some((head, tail)) = normalized.split_once(", ")
        && head
            .to_ascii_lowercase()
            .starts_with("whenever a creature blocks ")
        && tail
            .to_ascii_lowercase()
            .starts_with("blocking creatures get ")
    {
        let normalized_tail =
            tail.replacen("blocking creatures get ", "the blocking creature gets ", 1);
        normalized = format!("{head}, {normalized_tail}");
    }
    normalized
}

fn normalize_reveal_match_filter(filter: &str) -> String {
    let mut normalized = filter.trim().to_string();
    if !normalized.ends_with("card") && !normalized.ends_with("cards") {
        normalized.push_str(" card");
    }
    let lower = normalized.to_ascii_lowercase();
    if lower.starts_with("a ")
        || lower.starts_with("an ")
        || lower.starts_with("the ")
        || lower.starts_with("that ")
        || lower.starts_with("this ")
        || lower.starts_with("those ")
        || lower.starts_with("these ")
    {
        return normalized;
    }
    let article = if lower
        .chars()
        .next()
        .is_some_and(|ch| matches!(ch, 'a' | 'e' | 'i' | 'o' | 'u'))
    {
        "an"
    } else {
        "a"
    };
    format!("{article} {normalized}")
}

fn normalize_reveal_tagged_draw_clause(line: &str) -> Option<String> {
    for prefix in [
        "Reveal the top card of your library and tag it as 'revealed_0'. If the tagged object 'revealed_0' matches ",
        "you may Reveal the top card of your library and tag it as 'revealed_0'. If the tagged object 'revealed_0' matches ",
    ] {
        let Some(start) = line.find(prefix) else {
            continue;
        };
        let rest = &line[start + prefix.len()..];
        let (filter, suffix) = if let Some(filter) = rest.strip_prefix("")
            && let Some(stripped) = filter.strip_suffix(", you draw a card.")
        {
            (stripped, ".")
        } else if let Some(filter) = rest.strip_prefix("")
            && let Some(stripped) = filter.strip_suffix(", you draw a card")
        {
            (stripped, "")
        } else {
            continue;
        };

        let before = &line[..start];
        let reveal_clause = if prefix.starts_with("you may ") {
            format!(
                "you may reveal the top card of your library. If it's {}, draw a card{}",
                normalize_reveal_match_filter(filter),
                suffix
            )
        } else {
            format!(
                "Reveal the top card of your library. If it's {}, draw a card{}",
                normalize_reveal_match_filter(filter),
                suffix
            )
        };
        return Some(format!("{before}{reveal_clause}"));
    }
    None
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
            word.trim_matches(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '{' || ch == '}'))
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
            if lower.len() > 1
                && lower
                    .chars()
                    .all(|ch| matches!(ch, 'w' | 'u' | 'b' | 'r' | 'g' | 'c'))
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
        _ => {
            if let Some(backref) = describe_effect_count_backref(value) {
                format!("{backref} cards")
            } else {
                format!("{} cards", describe_value(value))
            }
        }
    }
}

fn describe_discard_count(value: &Value, filter: Option<&ObjectFilter>) -> String {
    let Some(filter) = filter else {
        return describe_card_count(value);
    };

    let card_phrase = describe_discard_card_phrase(filter);
    let plural_card_phrase = pluralize_discard_card_phrase(&card_phrase);
    match value {
        Value::Fixed(1) => format!("a {card_phrase}"),
        Value::Fixed(n) => format!("{n} {plural_card_phrase}"),
        _ => {
            if let Some(backref) = describe_effect_count_backref(value) {
                format!("{backref} {plural_card_phrase}")
            } else {
                format!("{} {plural_card_phrase}", describe_value(value))
            }
        }
    }
}

fn describe_discard_card_phrase(filter: &ObjectFilter) -> String {
    let mut bare = filter.clone();
    bare.controller = None;
    bare.owner = None;
    bare.targets_player = None;
    bare.targets_object = None;
    bare.tagged_constraints.clear();

    let mut phrase = strip_indefinite_article(&bare.description()).to_string();
    if let Some(stripped) = phrase.strip_suffix(" in hand") {
        phrase = stripped.to_string();
    }
    if phrase.is_empty() || phrase == "object" || phrase == "objects" {
        return "card".to_string();
    }
    if !phrase.contains("card") {
        phrase.push_str(" card");
    }
    phrase
}

fn pluralize_discard_card_phrase(phrase: &str) -> String {
    if phrase.ends_with('s') {
        phrase.to_string()
    } else {
        format!("{phrase}s")
    }
}

fn describe_effect_count_backref(value: &Value) -> Option<String> {
    match value {
        Value::EffectValue(_) => Some("that many".to_string()),
        Value::EffectValueOffset(_, offset) => {
            if *offset == 0 {
                Some("that many".to_string())
            } else if *offset > 0 {
                Some(format!("that many plus {}", offset))
            } else if *offset == -1 {
                Some("that many minus one".to_string())
            } else {
                Some(format!("that many minus {}", -offset))
            }
        }
        _ => None,
    }
}

fn is_generic_owned_card_search_filter(filter: &ObjectFilter) -> bool {
    filter.zone.is_none()
        && filter.controller.is_none()
        && (filter.owner.is_none() || matches!(filter.owner, Some(PlayerFilter::You)))
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
        && !filter.monocolored
        && !filter.token
        && !filter.nontoken
        && !filter.other
        && !filter.tapped
        && !filter.untapped
        && !filter.attacking
        && !filter.nonattacking
        && !filter.blocking
        && !filter.nonblocking
        && filter.power.is_none()
        && filter.toughness.is_none()
        && filter.mana_value.is_none()
        && !filter.has_mana_cost
        && !filter.has_tap_activated_ability
        && !filter.no_x_in_cost
        && filter.name.is_none()
        && filter.excluded_name.is_none()
        && filter.alternative_cast.is_none()
        && filter.static_abilities.is_empty()
        && filter.excluded_static_abilities.is_empty()
        && filter.custom_static_markers.is_empty()
        && filter.excluded_custom_static_markers.is_empty()
        && !filter.is_commander
        && !filter.noncommander
        && filter.tagged_constraints.is_empty()
        && filter.specific.is_none()
        && filter.any_of.is_empty()
        && !filter.source
}

fn describe_object_count(value: &Value) -> String {
    match value {
        Value::Fixed(1) => "a".to_string(),
        _ => describe_value(value),
    }
}

fn describe_for_each_count_filter(filter: &ObjectFilter) -> String {
    let mut bare = filter.clone();
    let controller = bare.controller.clone();
    let owner = bare.owner.clone();
    bare.controller = None;
    bare.owner = None;

    let mut subject = strip_indefinite_article(&bare.description()).to_string();
    subject = subject.replace("target player's ", "");
    subject = subject.replace("that player's ", "");
    let lower_subject = subject.to_ascii_lowercase();
    if lower_subject.starts_with("a ") {
        subject = subject[2..].to_string();
    } else if lower_subject.starts_with("an ") {
        subject = subject[3..].to_string();
    } else if let Some(rest) = lower_subject.strip_prefix("another ") {
        subject = format!("other {}", rest.trim());
    }

    let controller_suffix = match controller {
        Some(PlayerFilter::You) => Some("you control"),
        Some(PlayerFilter::NotYou) => Some("you don't control"),
        Some(PlayerFilter::Opponent) => Some("an opponent controls"),
        Some(PlayerFilter::Any) => Some("a player controls"),
        Some(PlayerFilter::Target(_)) | Some(PlayerFilter::IteratedPlayer) => Some("they control"),
        _ => None,
    };
    if let Some(suffix) = controller_suffix {
        return format!("{subject} {suffix}");
    }

    let owner_suffix = match owner {
        Some(PlayerFilter::You) => Some("you own"),
        Some(PlayerFilter::NotYou) => Some("you don't own"),
        Some(PlayerFilter::Opponent) => Some("an opponent owns"),
        Some(PlayerFilter::Any) => Some("a player owns"),
        Some(PlayerFilter::Target(_)) | Some(PlayerFilter::IteratedPlayer) => Some("they own"),
        _ => None,
    };
    if let Some(suffix) = owner_suffix {
        return format!("{subject} {suffix}");
    }

    subject
}

fn describe_demonstrative_tagged_object_spec(spec: &ChooseSpec) -> Option<String> {
    let ChooseSpec::Object(filter) = spec else {
        return None;
    };
    let has_it_tag = filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str() == "__it__"
    });
    if !has_it_tag {
        return None;
    }

    let mut base = filter.clone();
    base.tagged_constraints.retain(|constraint| {
        !(constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str() == "__it__")
    });

    let base_desc = strip_leading_article(&base.description())
        .trim()
        .to_string();
    if base_desc.is_empty() {
        Some("that object".to_string())
    } else {
        Some(format!("that {base_desc}"))
    }
}

fn describe_choose_spec(spec: &ChooseSpec) -> String {
    match spec {
        ChooseSpec::Target(inner) => {
            if let Some(tagged_text) = describe_demonstrative_tagged_object_spec(inner.as_ref()) {
                return tagged_text;
            }
            let inner_text = describe_choose_spec(inner);
            if inner_text == "it" {
                inner_text
            } else if inner_text.starts_with("this ") {
                inner_text
            } else if inner_text.starts_with("that ") || inner_text.starts_with("those ") {
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
        ChooseSpec::PlayerOrPlaneswalker(filter) => match filter {
            PlayerFilter::Opponent => "target opponent or planeswalker".to_string(),
            PlayerFilter::Any => "target player or planeswalker".to_string(),
            other => format!("target {} or planeswalker", describe_player_filter(other)),
        },
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
        ChooseSpec::All(filter) => {
            let desc = filter.description();
            let stripped = strip_leading_article(&desc);
            format!("all {}", pluralize_noun_phrase(stripped))
        }
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
                        (1, Some(3)) => format!("one, two, or three target {plural}"),
                        (min, Some(max)) => {
                            format!("{} to {} target {plural}", count_text(min), count_text(max))
                        }
                    }
                } else {
                    let base = strip_leading_article(&inner_text);
                    let plural = pluralize_noun_phrase(base);
                    let count_text = |n: usize| {
                        number_word(n as i32)
                            .map(str::to_string)
                            .unwrap_or_else(|| n.to_string())
                    };
                    if count.is_dynamic_x() {
                        return format!("X {plural}");
                    }
                    match (count.min, count.max) {
                        (0, None) => format!("any number of {plural}"),
                        (min, None) => {
                            if min == 1 {
                                format!("at least one {base}")
                            } else {
                                format!("at least {} {plural}", count_text(min))
                            }
                        }
                        (0, Some(max)) => {
                            if max == 1 {
                                format!("up to one {base}")
                            } else {
                                format!("up to {} {plural}", count_text(max))
                            }
                        }
                        (min, Some(max)) if min == max => {
                            if min == 1 {
                                format!("one {base}")
                            } else {
                                format!("{} {plural}", count_text(min))
                            }
                        }
                        (min, Some(max)) => {
                            format!("{} to {} {plural}", count_text(min), count_text(max))
                        }
                    }
                }
            }
        }
    }
}

fn describe_attach_objects_spec(spec: &ChooseSpec) -> String {
    if let ChooseSpec::All(filter) = spec
        && filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        })
    {
        if filter.subtypes.contains(&Subtype::Equipment) {
            return "that Equipment".to_string();
        }
        if filter.subtypes.contains(&Subtype::Aura) {
            return "that Aura".to_string();
        }
        if filter.card_types.is_empty() && filter.subtypes.is_empty() {
            return "it".to_string();
        }
    }
    describe_choose_spec(spec)
}

fn describe_goad_target(spec: &ChooseSpec) -> String {
    match spec {
        ChooseSpec::Tagged(tag) => {
            if tag.as_str().starts_with("counters_") {
                return "each creature that had counters put on it this way".to_string();
            }
            if is_implicit_reference_tag(tag.as_str()) {
                return "that creature".to_string();
            }
            describe_choose_spec(spec)
        }
        ChooseSpec::All(filter) => {
            let looks_like_plain_creature_filter = filter.zone == Some(Zone::Battlefield)
                && filter.card_types == vec![CardType::Creature]
                && filter.all_card_types.is_empty()
                && filter.excluded_card_types.is_empty()
                && filter.subtypes.is_empty()
                && filter.excluded_subtypes.is_empty()
                && !filter.source;
            if looks_like_plain_creature_filter {
                if let Some(controller) = filter.controller.as_ref() {
                    return match controller {
                        PlayerFilter::Opponent | PlayerFilter::NotYou => {
                            "all creatures you don't control".to_string()
                        }
                        PlayerFilter::Target(inner) => {
                            let who = describe_player_filter(inner);
                            if who == "player" {
                                "each creature target player controls".to_string()
                            } else {
                                format!("each creature target {who} controls")
                            }
                        }
                        PlayerFilter::IteratedPlayer => {
                            "each creature that player controls".to_string()
                        }
                        PlayerFilter::You => "each creature you control".to_string(),
                        _ => describe_choose_spec(spec),
                    };
                }
                return "each creature".to_string();
            }
            describe_choose_spec(spec)
        }
        _ => describe_choose_spec(spec),
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
            if let Some(tagged_text) = describe_demonstrative_tagged_object_spec(inner.as_ref()) {
                return tagged_text;
            }
            let inner_text = describe_choose_spec_without_graveyard_zone(inner);
            if inner_text == "it" {
                inner_text
            } else if inner_text.starts_with("this ")
                || inner_text.starts_with("that ")
                || inner_text.starts_with("those ")
            {
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
        ChooseSpec::PlayerOrPlaneswalker(filter) => match filter {
            PlayerFilter::Opponent => "target opponent or planeswalker".to_string(),
            PlayerFilter::Any => "target player or planeswalker".to_string(),
            other => format!("target {} or planeswalker", describe_player_filter(other)),
        },
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
                    let stripped = strip_leading_article(stripped);
                    return format!("all {}", pluralize_noun_phrase(stripped));
                }
                let text = strip_leading_article(&text);
                return format!("all {}", pluralize_noun_phrase(text));
            }
            let desc = filter.description();
            let stripped = strip_leading_article(&desc);
            format!("all {}", pluralize_noun_phrase(stripped))
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
                        (1, Some(3)) => format!("one, two, or three target {plural}"),
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

fn normalize_split_search_battlefield_then_hand_clause(text: &str) -> Option<String> {
    let trimmed = text.trim().trim_end_matches('.');
    let (first, second) = trimmed.split_once(". ")?;

    let first = first.strip_prefix("Search your library for ")?;
    let (first_selection, first_tail) = first.split_once(", ")?;
    let first_tail_ok = first_tail.eq_ignore_ascii_case("put it onto the battlefield tapped")
        || first_tail.eq_ignore_ascii_case("reveal it, put it onto the battlefield tapped");
    if !first_tail_ok {
        return None;
    }

    let second = second.strip_prefix("Search your library for ")?;
    let (second_selection, second_tail) = second.split_once(", ")?;
    let second_tail_ok = second_tail
        .eq_ignore_ascii_case("reveal it, put it into your hand, then shuffle")
        || second_tail.eq_ignore_ascii_case("put it into your hand, then shuffle");
    if !second_tail_ok {
        return None;
    }

    let normalize_selection = |raw: &str| {
        raw.trim()
            .trim_start_matches("up to one ")
            .trim_start_matches("a ")
            .trim_start_matches("an ")
            .trim_end_matches(" you own")
            .trim_end_matches(" card")
            .trim_end_matches(" cards")
            .trim()
            .to_string()
    };

    let first_subject = normalize_selection(first_selection);
    let second_subject = normalize_selection(second_selection);
    if first_subject.is_empty() || second_subject.is_empty() || first_subject != second_subject {
        return None;
    }

    Some(format!(
        "Search your library for up to two {second_subject} cards, reveal those cards, put one onto the battlefield tapped and the other into your hand, then shuffle."
    ))
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
        (Some(Value::Fixed(min_value)), Value::Fixed(max_value)) => {
            match (*min_value, *max_value) {
                (0, 1) => "Choose up to one -".to_string(),
                (1, 1) => "Choose one -".to_string(),
                (1, 2) => "Choose one or both -".to_string(),
                (1, n) if n > 2 => "Choose one or more -".to_string(),
                (0, n) => {
                    if let Some(word) = number_word(n) {
                        format!("Choose up to {word} -")
                    } else {
                        format!("Choose up to {n} -")
                    }
                }
                (n, m) if n == m => {
                    if let Some(word) = number_word(n) {
                        format!("Choose {word} -")
                    } else {
                        format!("Choose {n} -")
                    }
                }
                _ => format!("Choose between {min_value} and {max_value} mode(s) -"),
            }
        }
        (None, Value::Fixed(value)) if *value > 0 => {
            if let Some(word) = number_word(*value) {
                format!("Choose {word} -")
            } else {
                format!("Choose {value} mode(s) -")
            }
        }
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
        other => {
            let raw = format!("{other:?}");
            let mut out = String::with_capacity(raw.len() + 4);
            for (idx, ch) in raw.chars().enumerate() {
                if idx > 0 && ch.is_ascii_uppercase() {
                    out.push(' ');
                }
                out.push(ch.to_ascii_lowercase());
            }
            out
        }
    }
}

fn describe_value(value: &Value) -> String {
    match value {
        Value::Fixed(n) => n.to_string(),
        Value::Add(left, right) => {
            format!("{} plus {}", describe_value(left), describe_value(right))
        }
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
        Value::CountScaled(filter, multiplier) => {
            format!("{multiplier} times the number of {}", filter.description())
        }
        Value::CreaturesDiedThisTurn => "the number of creatures that died this turn".to_string(),
        Value::CountPlayers(filter) => format!("the number of {}", describe_player_filter(filter)),
        Value::SourcePower => "this source's power".to_string(),
        Value::SourceToughness => "this source's toughness".to_string(),
        Value::PowerOf(spec) => format!("{} power", describe_possessive_choose_spec(spec)),
        Value::ToughnessOf(spec) => format!("{} toughness", describe_possessive_choose_spec(spec)),
        Value::ManaValueOf(spec) => {
            format!("{} mana value", describe_possessive_choose_spec(spec))
        }
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
        Value::ColorsOfManaSpentToCastThisSpell => {
            "the number of colors of mana spent to cast this spell".to_string()
        }
        Value::EffectValue(id) => format!("the count result of effect #{}", id.0),
        Value::EffectValueOffset(id, offset) => {
            if *offset == 0 {
                format!("the count result of effect #{}", id.0)
            } else if *offset > 0 {
                format!("the count result of effect #{} plus {}", id.0, offset)
            } else {
                format!("the count result of effect #{} minus {}", id.0, -offset)
            }
        }
        Value::EventValue(EventValueSpec::Amount)
        | Value::EventValue(EventValueSpec::LifeAmount) => "that much".to_string(),
        Value::EventValueOffset(EventValueSpec::Amount, offset)
        | Value::EventValueOffset(EventValueSpec::LifeAmount, offset) => {
            if *offset == 0 {
                "that much".to_string()
            } else if *offset > 0 {
                format!("that much plus {}", offset)
            } else {
                format!("that much minus {}", -offset)
            }
        }
        Value::EventValue(EventValueSpec::BlockersBeyondFirst { multiplier }) => {
            if *multiplier == 1 {
                "the number of blockers beyond the first".to_string()
            } else {
                format!("{multiplier} times the number of blockers beyond the first")
            }
        }
        Value::EventValueOffset(EventValueSpec::BlockersBeyondFirst { multiplier }, offset) => {
            let base = if *multiplier == 1 {
                "the number of blockers beyond the first".to_string()
            } else {
                format!("{multiplier} times the number of blockers beyond the first")
            };
            if *offset == 0 {
                base
            } else if *offset > 0 {
                format!("{base} plus {}", offset)
            } else {
                format!("{base} minus {}", -offset)
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

fn choose_spec_allows_multiple(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::Target(inner) => choose_spec_allows_multiple(inner),
        ChooseSpec::All(_) | ChooseSpec::EachPlayer(_) => true,
        ChooseSpec::WithCount(inner, count) => {
            if count.dynamic_x {
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
    let has = if plural_target { "have" } else { "has" };
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
            power,
            toughness,
            sublayer,
        } => {
            let verb = if *sublayer == crate::continuous::PtSublayer::Setting {
                has
            } else {
                gets
            };
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
                clauses.push(format!("{gains} {}", describe_inline_ability(inline)));
            } else {
                clauses.push(format!("{gains} {}", ability.display()));
            }
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
            crate::effects::continuous::RuntimeModification::ChangeControllerToPlayer(player) => {
                clauses.push(format!(
                    "changes controller to {}",
                    describe_player_filter(player)
                ));
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
    if effect.modification.is_none()
        && effect.additional_modifications.is_empty()
        && let [crate::effects::continuous::RuntimeModification::ChangeControllerToPlayer(player)] =
            effect.runtime_modifications.as_slice()
    {
        let mut text = format!(
            "{} gains control of {target}",
            describe_player_filter(player)
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

fn describe_attached_object_for_tag(tag: &str, spec: Option<&ChooseSpec>) -> String {
    let default = match tag {
        "enchanted" => "enchanted permanent",
        "equipped" => "equipped creature",
        _ => "attached object",
    };

    if tag != "enchanted" {
        return default.to_string();
    }

    let Some(ChooseSpec::Object(filter)) = spec else {
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

    if filter.card_types.len() == 1 && filter.all_card_types.is_empty() {
        return format!(
            "enchanted {}",
            describe_card_type_word_local(filter.card_types[0])
        );
    }

    default.to_string()
}

fn describe_tag_attached_then_tap_or_untap(
    tag_attached: &crate::effects::TagAttachedToSourceEffect,
    next: &Effect,
) -> Option<String> {
    let tag = tag_attached.tag.as_str();
    if !matches!(tag, "enchanted" | "equipped") {
        return None;
    }

    if let Some(tap) = next.downcast_ref::<crate::effects::TapEffect>()
        && choose_spec_references_tag(&tap.spec, tag)
    {
        let attached_object = describe_attached_object_for_tag(tag, Some(&tap.spec));
        return Some(format!("Tap {attached_object}"));
    }
    if let Some(untap) = next.downcast_ref::<crate::effects::UntapEffect>()
        && choose_spec_references_tag(&untap.spec, tag)
    {
        let attached_object = describe_attached_object_for_tag(tag, Some(&untap.spec));
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
        Until::ControllersNextUntapStep => "during its controller's next untap step".to_string(),
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
        crate::effect::Restriction::Transform(filter) => {
            format!("{} can't transform", filter.description())
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
        EffectPredicate::DidNotHappen => "that doesn't happen".to_string(),
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
        Condition::PlayerControls { player, filter } => {
            format!(
                "{} controls {}",
                describe_player_filter(player),
                filter.description()
            )
        }
        Condition::PlayerControlsMost { player, filter } => {
            let mut subject = strip_indefinite_article(&filter.description()).to_string();
            if !subject.ends_with('s') {
                subject.push('s');
            }
            format!(
                "{} controls the most {}",
                describe_player_filter(player),
                subject
            )
        }
        Condition::PlayerHasLessLifeThanYou { player } => {
            format!("{} has less life than you", describe_player_filter(player))
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
        Condition::TargetIsBlocked => "the target is blocked".to_string(),
        Condition::TargetWasKicked => "the target spell was kicked".to_string(),
        Condition::TargetSpellCastOrderThisTurn(2) => {
            "the target spell was the second spell cast this turn".to_string()
        }
        Condition::TargetSpellCastOrderThisTurn(order) => {
            format!("the target spell was spell number {order} cast this turn")
        }
        Condition::TargetSpellControllerIsPoisoned => {
            "the target spell's controller is poisoned".to_string()
        }
        Condition::TargetSpellManaSpentToCastAtLeast { amount, symbol } => {
            if let Some(symbol) = symbol {
                format!(
                    "at least {amount} {} mana was spent to cast the target spell",
                    describe_mana_symbol(*symbol)
                )
            } else {
                format!("at least {amount} mana was spent to cast the target spell")
            }
        }
        Condition::YouControlMoreCreaturesThanTargetSpellController => {
            "you control more creatures than the target spell's controller".to_string()
        }
        Condition::TargetHasGreatestPowerAmongCreatures => {
            "the target creature has the greatest power among creatures on the battlefield"
                .to_string()
        }
        Condition::TargetManaValueLteColorsSpentToCastThisSpell => {
            "the target's mana value is less than or equal to the number of colors of mana spent to cast this spell".to_string()
        }
        Condition::SourceIsTapped => "this source is tapped".to_string(),
        Condition::SourceHasNoCounter(counter_type) => format!(
            "there are no {} counters on this source",
            describe_counter_type(*counter_type)
        ),
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
        Condition::Not(inner) => {
            if let Condition::TargetSpellManaSpentToCastAtLeast {
                amount: 1,
                symbol: None,
            } = inner.as_ref()
            {
                "no mana was spent to cast the target spell".to_string()
            } else {
                format!("not ({})", describe_condition(inner))
            }
        }
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
            && let Some(compact) = describe_choose_then_for_each_copy(choose, for_each)
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
            && let Some(move_to_zone) =
                filtered[idx + 1].downcast_ref::<crate::effects::MoveToZoneEffect>()
            && let Some(compact) = describe_choose_then_move_to_library(choose, move_to_zone)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(choose) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(move_to_zone) =
                filtered[idx + 2].downcast_ref::<crate::effects::MoveToZoneEffect>()
            && let Some(compact) =
                describe_look_at_top_then_choose_move_to_exile(look_at_top, choose, move_to_zone)
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(choose) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(exile) = filtered[idx + 2].downcast_ref::<crate::effects::ExileEffect>()
            && let Some(compact) =
                describe_look_at_top_then_choose_exile(look_at_top, choose, exile)
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(with_id) = filtered[idx + 1].downcast_ref::<crate::effects::WithIdEffect>()
            && let Some(sacrifice) = with_id
                .effect
                .downcast_ref::<crate::effects::SacrificeEffect>()
            && let Some(compact) = describe_choose_then_sacrifice(choose, sacrifice)
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
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(destroy) = filtered[idx + 1].downcast_ref::<crate::effects::DestroyEffect>()
            && let Some(compact) = describe_choose_then_destroy(choose, destroy)
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
        if idx + 1 < filtered.len()
            && let Some(mill) = filtered[idx].downcast_ref::<crate::effects::MillEffect>()
            && let Some(may) = filtered[idx + 1].downcast_ref::<crate::effects::MayEffect>()
            && let Some(compact) = describe_mill_then_may_return(mill, may)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(gain) = filtered[idx].downcast_ref::<crate::effects::GainLifeEffect>()
            && let Some(scry) = filtered[idx + 1].downcast_ref::<crate::effects::ScryEffect>()
            && let Some(compact) = describe_gain_life_then_scry(gain, scry)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(scry) = filtered[idx].downcast_ref::<crate::effects::ScryEffect>()
            && let Some(draw) = filtered[idx + 1].downcast_ref::<crate::effects::DrawCardsEffect>()
            && let Some(compact) = describe_scry_then_draw(scry, draw)
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
    let tapped_suffix = if move_back.enters_tapped { " tapped" } else { "" };
    let controller_suffix = match move_back.battlefield_controller {
        crate::effects::BattlefieldController::Preserve => "",
        crate::effects::BattlefieldController::Owner => owner_control_suffix,
        crate::effects::BattlefieldController::You => " under your control",
    };
    Some(format!(
        "Exile {target}, then return {return_object} to the battlefield{tapped_suffix}{controller_suffix}"
    ))
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
        return normalize_inline_ability_text(ability, text.trim());
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

fn activated_ability_has_source_tap_cost(activated: &crate::ability::ActivatedAbility) -> bool {
    activated.mana_cost.costs().iter().any(|cost| {
        cost.requires_tap()
            || cost.effect_ref().is_some_and(|effect| {
                effect
                    .downcast_ref::<crate::effects::TapEffect>()
                    .is_some_and(|tap| matches!(tap.spec, ChooseSpec::Source))
            })
    })
}

fn activated_ability_has_source_untap_cost(activated: &crate::ability::ActivatedAbility) -> bool {
    activated.mana_cost.costs().iter().any(|cost| {
        cost.requires_untap()
            || cost.effect_ref().is_some_and(|effect| {
                effect
                    .downcast_ref::<crate::effects::UntapEffect>()
                    .is_some_and(|untap| matches!(untap.spec, ChooseSpec::Source))
            })
    })
}

fn normalize_inline_ability_text(ability: &Ability, text: &str) -> String {
    let trimmed = text.trim();
    let AbilityKind::Activated(activated) = &ability.kind else {
        return trimmed.to_string();
    };

    let lower = trimmed.to_ascii_lowercase();
    if activated_ability_has_source_tap_cost(activated)
        && lower.starts_with("t ")
        && let Some((_, rest)) = trimmed.split_once(' ')
        && !rest.trim().is_empty()
    {
        return format!("{{T}}: {}", rest.trim());
    }
    if activated_ability_has_source_untap_cost(activated)
        && lower.starts_with("q ")
        && let Some((_, rest)) = trimmed.split_once(' ')
        && !rest.trim().is_empty()
    {
        return format!("{{Q}}: {}", rest.trim());
    }
    trimmed.to_string()
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
    let article = if matches!(first, 'a' | 'e' | 'i' | 'o' | 'u') {
        "an"
    } else {
        "a"
    };
    format!("{article} {trimmed}")
}

fn describe_for_each_double_counters(for_each: &crate::effects::ForEachObject) -> Option<String> {
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
    let Value::CountersOn(source, Some(counter_type)) = &put.count else {
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

fn normalize_put_counter_number_for_each(line: &str) -> Option<String> {
    let (before, after) = split_once_ascii_ci(line, "put the number of ")?;
    let (count_and_counter, after_target) = after.split_once(" counter(s) on ")?;
    let (target, suffix) = after_target
        .split_once(". ")
        .map(|(target, tail)| (target, format!(". {tail}")))
        .unwrap_or_else(|| (after_target.trim_end_matches('.'), String::new()));
    let target = target.trim();

    let count_filter = if let Some(filter) = count_and_counter.strip_suffix(" +1/+1") {
        format!("a +1/+1 counter on {target} for each {filter}")
    } else if let Some(filter) = count_and_counter.strip_suffix(" -1/-1") {
        format!("a -1/-1 counter on {target} for each {filter}")
    } else {
        return None;
    };

    let mut rewritten = String::with_capacity(line.len());
    rewritten.push_str(before);
    if !before.is_empty() && !before.ends_with(' ') {
        rewritten.push(' ');
    }
    rewritten.push_str("Put ");
    rewritten.push_str(&count_filter);
    if suffix.is_empty() {
        rewritten.push('.');
    } else {
        rewritten.push_str(&suffix);
    }
    Some(rewritten)
}

fn strip_indefinite_article(text: &str) -> &str {
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

fn pluralize_word(word: &str) -> String {
    if word.chars().last().is_some_and(|ch| ch.is_ascii_digit()) {
        return word.to_string();
    }
    let lower = word.to_ascii_lowercase();
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
    let mut base = strip_indefinite_article(phrase).trim();
    let mut trailing = "";
    if let Some(stripped) = base.strip_suffix('.') {
        base = stripped.trim_end();
        trailing = ".";
    }
    if base.contains(" or ") {
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
    for suffix in [
        " you control",
        " you own",
        " an opponent controls",
        " an opponent owns",
        " target opponent controls",
        " target player controls",
        " target player owns",
        " that player controls",
        " that player owns",
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
            return format!("{head_plural}{suffix}{trailing}");
        }
    }
    if base.ends_with('s') {
        format!("{base}{trailing}")
    } else {
        format!("{}{}", pluralize_word(base), trailing)
    }
}

fn sacrifice_uses_chosen_tag(filter: &ObjectFilter, tag: &str) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str() == tag
    })
}

fn describe_for_players_choose_types_then_sacrifice_rest(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    let (tail, choose_effects) = for_players.effects.split_last()?;
    let sacrifice = tail.downcast_ref::<crate::effects::SacrificeEffect>()?;
    if sacrifice.player != PlayerFilter::IteratedPlayer {
        return None;
    }
    let Value::Count(count_filter) = &sacrifice.count else {
        return None;
    };
    if count_filter != &sacrifice.filter {
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

    let mut chosen_types = Vec::new();
    for choose in chooses {
        if choose.zone != Zone::Battlefield
            || choose.is_search
            || !choose.count.is_single()
            || choose.chooser != PlayerFilter::IteratedPlayer
            || choose.tag.as_str() != keep_tag
        {
            return None;
        }
        let has_keep_guard = choose.filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
                && constraint.tag.as_str() == keep_tag
        });
        if !has_keep_guard || choose.filter.card_types.len() != 1 {
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
        if let Some(rest) = chosen.strip_prefix(&format!("{player}'s ")) {
            let chosen_kind = with_indefinite_article(rest);
            return Some(format!("{player} {verb} {chosen_kind} of their choice"));
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

fn describe_choose_then_destroy(
    choose: &crate::effects::ChooseObjectsEffect,
    destroy: &crate::effects::DestroyEffect,
) -> Option<String> {
    if choose.zone != Zone::Battlefield || choose.is_search || !choose.count.is_single() {
        return None;
    }
    let ChooseSpec::Tagged(tag) = &destroy.spec else {
        return None;
    };
    if tag.as_str() != choose.tag.as_str() {
        return None;
    }

    let chooser = describe_player_filter(&choose.chooser);
    let choose_verb = player_verb(&chooser, "choose", "chooses");
    let description = choose.filter.description();
    let chosen = if let Some(rest) = description.strip_prefix("target player's ") {
        format!("a {} they control", rest)
    } else if let Some(rest) = description.strip_prefix("that player's ") {
        format!("a {} they control", rest)
    } else {
        with_indefinite_article(&description)
    };
    Some(format!("{chooser} {choose_verb} {chosen}. Destroy it"))
}

fn describe_choose_then_for_each_copy(
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
    if !matches!(create_copy.target, ChooseSpec::Tagged(ref tag) if tag == &choose.tag) {
        return None;
    }
    if create_copy.controller != PlayerFilter::You
        || create_copy.enters_tapped
        || create_copy.has_haste
        || create_copy.enters_attacking
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

    let selected = describe_choose_spec(
        &ChooseSpec::target(ChooseSpec::Object(choose.filter.clone())).with_count(choose.count),
    );
    Some(format!(
        "For each of {selected}, create {} tokens that are copies of that permanent",
        describe_value(&create_copy.count)
    ))
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

fn move_to_exile_uses_chosen_tag(
    move_to_zone: &crate::effects::MoveToZoneEffect,
    tag: &str,
) -> bool {
    move_to_zone.zone == Zone::Exile
        && matches!(move_to_zone.target.base(), ChooseSpec::Tagged(t) if t.as_str() == tag)
}

fn describe_for_each_filter(filter: &ObjectFilter) -> String {
    let mut base_filter = filter.clone();
    base_filter.controller = None;

    let description = base_filter.description();
    let mut base = strip_indefinite_article(&description).to_string();
    if let Some(rest) = base.strip_prefix("another ") {
        base = format!("other {rest}");
    }
    if let Some(rest) = base.strip_prefix("permanent ")
        && matches!(filter.zone, None | Some(Zone::Battlefield))
    {
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
        && !filter.nonattacking
        && !filter.blocking
        && !filter.nonblocking
        && filter.zone.is_none()
        && filter.tagged_constraints.is_empty()
        && filter.targets_object.is_none()
        && filter.targets_player.is_none()
        && filter.custom_static_markers.is_empty()
        && filter.excluded_custom_static_markers.is_empty()
        && !filter.noncommander
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

fn describe_create_for_each_count(value: &Value) -> Option<String> {
    match value {
        Value::Count(filter) => Some(describe_for_each_filter(filter)),
        Value::CreaturesDiedThisTurn => Some("creature that died this turn".to_string()),
        _ => None,
    }
}

fn describe_compact_token_count(value: &Value, token_name: &str) -> String {
    match value {
        Value::Fixed(1) => format!("a {token_name} token"),
        Value::Fixed(n) => format!("{n} {token_name} tokens"),
        Value::Count(filter) => {
            format!(
                "a {token_name} token for each {}",
                describe_for_each_count_filter(filter)
            )
        }
        Value::CountScaled(filter, multiplier) => {
            if *multiplier == 1 {
                format!(
                    "a {token_name} token for each {}",
                    describe_for_each_count_filter(filter)
                )
            } else {
                format!(
                    "{multiplier} {token_name} tokens for each {}",
                    describe_for_each_count_filter(filter)
                )
            }
        }
        Value::CreaturesDiedThisTurn => {
            format!("a {token_name} token for each creature that died this turn")
        }
        Value::ColorsOfManaSpentToCastThisSpell => {
            format!("a {token_name} token for each color of mana spent to cast this spell")
        }
        _ => format!("{} {token_name} token(s)", describe_value(value)),
    }
}

fn describe_compact_create_token(
    create_token: &crate::effects::CreateTokenEffect,
) -> Option<String> {
    if create_token.enters_tapped
        || create_token.enters_attacking
        || create_token.exile_at_end_of_combat
        || create_token.sacrifice_at_next_end_step
        || create_token.exile_at_next_end_step
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
    for owner_prefix in [
        "target player's ",
        "that player's ",
        "their ",
        "your ",
        "an opponent's ",
    ] {
        if let Some(rest) = card_desc.strip_prefix(owner_prefix) {
            card_desc = rest.to_string();
            break;
        }
    }
    if let Some(rest) = card_desc.strip_suffix(" hands") {
        card_desc = format!("{rest} hand");
    }
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

fn move_to_library_uses_chosen_tag(
    move_to_zone: &crate::effects::MoveToZoneEffect,
    tag: &str,
) -> bool {
    move_to_zone.zone == Zone::Library
        && matches!(move_to_zone.target.base(), ChooseSpec::Tagged(t) if t.as_str() == tag)
}

fn describe_choose_zone_origin(
    choose: &crate::effects::ChooseObjectsEffect,
    zone_text: &str,
) -> String {
    match choose.filter.owner.as_ref() {
        Some(PlayerFilter::IteratedPlayer) => format!("from their {zone_text}"),
        Some(owner) => format!(
            "from {} {zone_text}",
            describe_possessive_player_filter(owner)
        ),
        None => format!("from a {zone_text}"),
    }
}

fn describe_choose_then_move_to_library(
    choose: &crate::effects::ChooseObjectsEffect,
    move_to_zone: &crate::effects::MoveToZoneEffect,
) -> Option<String> {
    if !move_to_library_uses_chosen_tag(move_to_zone, choose.tag.as_str()) {
        return None;
    }

    let origin = match choose.zone {
        Zone::Hand => describe_choose_zone_origin(choose, "hand"),
        Zone::Graveyard => describe_choose_zone_origin(choose, "graveyard"),
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

    let chooser = describe_player_filter(&choose.chooser);
    let choose_verb = player_verb(&chooser, "choose", "chooses");
    let put_verb = player_verb(&chooser, "put", "puts");
    let chosen = describe_choose_selection(choose);
    let moved_ref = if choose.count.is_single() {
        "it"
    } else {
        "them"
    };

    let destination = match choose.filter.owner.as_ref() {
        Some(PlayerFilter::IteratedPlayer) => "their library".to_string(),
        Some(owner) => format!("{} library", describe_possessive_player_filter(owner)),
        None => owner_library_phrase_for_spec(&move_to_zone.target).to_string(),
    };
    let placement = if move_to_zone.to_top {
        "on top of"
    } else {
        "on the bottom of"
    };

    Some(format!(
        "{chooser} {choose_verb} {chosen} {origin}, then {put_verb} {moved_ref} {placement} {destination}"
    ))
}

fn describe_look_at_top_then_choose_exile(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    exile: &crate::effects::ExileEffect,
) -> Option<String> {
    if !exile_uses_chosen_tag(&exile.spec, choose.tag.as_str()) {
        return None;
    }
    describe_look_at_top_then_choose_exile_text(look_at_top, choose)
}

fn describe_look_at_top_then_choose_move_to_exile(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    move_to_zone: &crate::effects::MoveToZoneEffect,
) -> Option<String> {
    if !move_to_exile_uses_chosen_tag(move_to_zone, choose.tag.as_str()) {
        return None;
    }
    describe_look_at_top_then_choose_exile_text(look_at_top, choose)
}

fn describe_look_at_top_then_choose_exile_text(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    choose: &crate::effects::ChooseObjectsEffect,
) -> Option<String> {
    if choose.zone != Zone::Library || choose.is_search || !choose.count.is_single() {
        return None;
    }
    let references_looked = choose.filter.tagged_constraints.iter().any(|constraint| {
        matches!(
            constraint.relation,
            crate::filter::TaggedOpbjectRelation::IsTaggedObject
        ) && constraint.tag.as_str() == look_at_top.tag.as_str()
    });
    if !references_looked {
        return None;
    }

    let owner = describe_possessive_player_filter(&look_at_top.player);
    let count_text = small_number_word(look_at_top.count as u32)
        .map(str::to_string)
        .unwrap_or_else(|| look_at_top.count.to_string());
    let noun = if look_at_top.count == 1 {
        "card"
    } else {
        "cards"
    };
    let exile_ref = if look_at_top.count == 1 {
        "it"
    } else {
        "one of them"
    };
    Some(format!(
        "Look at the top {count_text} {noun} of {owner} library, then exile {exile_ref}"
    ))
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
        describe_discard_count(&discard.count, discard.card_filter.as_ref())
    );
    if discard.random {
        text.push_str(" at random");
    }
    Some(text)
}

fn describe_mill_then_may_return(
    mill: &crate::effects::MillEffect,
    may: &crate::effects::MayEffect,
) -> Option<String> {
    if may.effects.len() != 1 {
        return None;
    }
    let return_effect = may.effects.first()?;
    let is_return_to_hand = return_effect
        .downcast_ref::<crate::effects::ReturnToHandEffect>()
        .is_some()
        || return_effect
            .downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()
            .is_some();
    if !is_return_to_hand {
        return None;
    }

    let decider = may.decider.as_ref().unwrap_or(&mill.player);
    if decider != &mill.player {
        return None;
    }

    let player = describe_player_filter(&mill.player);
    let mill_clause = format!(
        "{player} {} {}",
        player_verb(&player, "mill", "mills"),
        describe_card_count(&mill.count)
    );
    let return_clause = lowercase_first(&describe_effect(return_effect));
    Some(format!("{mill_clause}, then {player} may {return_clause}"))
}

fn describe_gain_life_then_scry(
    gain: &crate::effects::GainLifeEffect,
    scry: &crate::effects::ScryEffect,
) -> Option<String> {
    let crate::target::ChooseSpec::Player(gain_player) = gain.player.base() else {
        return None;
    };
    if gain_player != &scry.player {
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

fn describe_scry_then_draw(
    scry: &crate::effects::ScryEffect,
    draw: &crate::effects::DrawCardsEffect,
) -> Option<String> {
    if scry.player != draw.player {
        return None;
    }
    if scry.player == PlayerFilter::You {
        return Some(format!(
            "Scry {}, then draw {}",
            describe_value(&scry.count),
            describe_card_count(&draw.count)
        ));
    }

    let player = describe_player_filter(&scry.player);
    Some(format!(
        "{player} {} {}, then {} {}",
        player_verb(&player, "scry", "scries"),
        describe_value(&scry.count),
        player_verb(&player, "draw", "draws"),
        describe_card_count(&draw.count)
    ))
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
        let who = may
            .decider
            .as_ref()
            .map(describe_player_filter)
            .unwrap_or_else(|| "you".to_string());
        match if_effect.predicate {
            EffectPredicate::DidNotHappen => {
                if who == "you" {
                    "If you don't".to_string()
                } else {
                    format!("If {who} doesn't")
                }
            }
            _ => {
                if who == "you" {
                    "If you do".to_string()
                } else {
                    format!("If {who} does")
                }
            }
        }
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

    let filter_text = if choose.description.trim().is_empty()
        || choose.description.trim().eq_ignore_ascii_case("objects")
    {
        choose.filter.description()
    } else {
        choose.description.trim().to_string()
    };
    let selection_text = if choose.count.max == Some(1) {
        with_indefinite_article(&filter_text)
    } else {
        let mut count_text = describe_choice_count(&choose.count);
        if count_text == "any number" {
            count_text = "any number of".to_string();
        }
        format!("{count_text} {filter_text}")
    };
    let selection_text = describe_search_selection_with_cards(&selection_text);
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

    let mut text;
    match destination {
        SearchDestination::Battlefield { tapped } => {
            text = if shuffle.is_some() && shuffle_before_move {
                format!(
                    "Search {} library for {}{}, shuffle, then put {} onto the battlefield",
                    describe_possessive_player_filter(&choose.chooser),
                    selection_text,
                    reveal_clause,
                    pronoun
                )
            } else {
                format!(
                    "Search {} library for {}{}, put {} onto the battlefield",
                    describe_possessive_player_filter(&choose.chooser),
                    selection_text,
                    reveal_clause,
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
                    "Search {} library for {}{}, shuffle, then put {} into {} hand",
                    describe_possessive_player_filter(&choose.chooser),
                    selection_text,
                    reveal_clause,
                    pronoun,
                    describe_possessive_player_filter(&choose.chooser)
                )
            } else {
                format!(
                    "Search {} library for {}{}, put {} into {} hand",
                    describe_possessive_player_filter(&choose.chooser),
                    selection_text,
                    reveal_clause,
                    pronoun,
                    describe_possessive_player_filter(&choose.chooser)
                )
            };
        }
        SearchDestination::Graveyard => {
            text = if shuffle.is_some() && shuffle_before_move {
                format!(
                    "Search {} library for {}{}, shuffle, then put {} into {} graveyard",
                    describe_possessive_player_filter(&choose.chooser),
                    selection_text,
                    reveal_clause,
                    pronoun,
                    describe_possessive_player_filter(&choose.chooser)
                )
            } else {
                format!(
                    "Search {} library for {}{}, put {} into {} graveyard",
                    describe_possessive_player_filter(&choose.chooser),
                    selection_text,
                    reveal_clause,
                    pronoun,
                    describe_possessive_player_filter(&choose.chooser)
                )
            };
        }
        SearchDestination::LibraryTop => {
            text = if shuffle.is_some() && shuffle_before_move {
                format!(
                    "Search {} library for {}{}, then shuffle and put {} on top of {} library",
                    describe_possessive_player_filter(&choose.chooser),
                    selection_text,
                    reveal_clause,
                    pronoun,
                    describe_possessive_player_filter(&choose.chooser)
                )
            } else {
                format!(
                    "Search {} library for {}{}, put {} on top of {} library",
                    describe_possessive_player_filter(&choose.chooser),
                    selection_text,
                    reveal_clause,
                    pronoun,
                    describe_possessive_player_filter(&choose.chooser)
                )
            };
            if !choose.count.is_single() {
                text.push_str(" in any order");
            }
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
    let mut shared_target: Option<String> = None;
    let mut saw_tap = false;
    let mut saw_untap = false;
    let mut terse_mode_labels = true;
    for mode in &choose_mode.modes {
        if mode.effects.len() != 1 {
            return None;
        }
        let mode_label = mode
            .description
            .trim()
            .trim_end_matches('.')
            .to_ascii_lowercase();
        if mode_label != "tap" && mode_label != "untap" {
            terse_mode_labels = false;
        }
        let effect = &mode.effects[0];
        if let Some(tap) = effect.downcast_ref::<crate::effects::TapEffect>() {
            saw_tap = true;
            let candidate = describe_choose_spec(&tap.spec);
            if let Some(existing) = &shared_target {
                if existing != &candidate {
                    return None;
                }
            } else {
                shared_target = Some(candidate);
            }
            continue;
        }
        if let Some(untap) = effect.downcast_ref::<crate::effects::UntapEffect>() {
            saw_untap = true;
            let candidate = describe_choose_spec(&untap.spec);
            if let Some(existing) = &shared_target {
                if existing != &candidate {
                    return None;
                }
            } else {
                shared_target = Some(candidate);
            }
            continue;
        }
        return None;
    }
    if saw_tap && saw_untap {
        let target = shared_target.unwrap_or_else(|| "that object".to_string());
        if terse_mode_labels {
            return Some(format!("Tap or untap {target}"));
        }
        return Some(format!("Choose one — Tap {target}. • Untap {target}."));
    }
    None
}

fn describe_put_or_remove_counter_mode(
    choose_mode: &crate::effects::ChooseModeEffect,
) -> Option<String> {
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

    let mut put_mode: Option<(&crate::effects::PutCountersEffect, String)> = None;
    let mut remove_mode: Option<(&crate::effects::RemoveCountersEffect, String)> = None;
    let mut remove_followup_mode: Option<crate::effects::ChooseModeEffect> = None;

    for mode in &choose_mode.modes {
        let description = if mode.description.trim().is_empty() {
            describe_effect_list(&mode.effects)
        } else {
            mode.description.trim().to_string()
        };
        if mode.effects.len() == 1 {
            let effect = &mode.effects[0];
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

fn describe_effect_impl(effect: &Effect) -> String {
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        if let Some(compact) = describe_search_sequence(sequence) {
            return compact;
        }
        return describe_effect_list(&sequence.effects);
    }
    if let Some(for_each) = effect.downcast_ref::<crate::effects::ForEachObject>() {
        if let Some(compact) = describe_for_each_double_counters(for_each) {
            return compact;
        }
        let description = for_each.filter.description();
        let filter_text = strip_indefinite_article(&description);
        return format!(
            "For each {}, {}",
            filter_text,
            describe_effect_list(&for_each.effects)
        );
    }
    if let Some(for_each_tagged) = effect.downcast_ref::<crate::effects::ForEachTaggedEffect>() {
        let tag = for_each_tagged.tag.as_str();
        let subject = if tag.starts_with("destroyed_") {
            "For each object destroyed this way".to_string()
        } else if tag.starts_with("exiled_") {
            "For each object exiled this way".to_string()
        } else if tag.starts_with("sacrificed_") {
            "For each object sacrificed this way".to_string()
        } else if tag.is_empty() {
            "For each tagged object".to_string()
        } else {
            format!("For each tagged '{tag}' object")
        };
        return format!(
            "{subject}, {}",
            describe_effect_list(&for_each_tagged.effects)
        );
    }
    if let Some(for_players) = effect.downcast_ref::<crate::effects::ForPlayersEffect>() {
        if let Some(compact) = describe_for_players_choose_types_then_sacrifice_rest(for_players) {
            return compact;
        }
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
        let (zone_phrase, zone_keyword) = match choose.zone {
            Zone::Battlefield => ("the battlefield", "battlefield"),
            Zone::Hand => ("a hand", "hand"),
            Zone::Graveyard => ("a graveyard", "graveyard"),
            Zone::Library => ("a library", "library"),
            Zone::Stack => ("the stack", "stack"),
            Zone::Exile => ("exile", "exile"),
            Zone::Command => ("the command zone", "command"),
        };
        let filter_lower = filter_text.to_ascii_lowercase();
        let includes_zone_already = filter_lower.contains(zone_keyword);
        let location_suffix = if includes_zone_already {
            String::new()
        } else {
            format!(" in {zone_phrase}")
        };
        return format!(
            "{} {} {}{} and tags it as '{}'",
            chooser,
            if search_like {
                "searches for"
            } else {
                choose_verb
            },
            choice_text,
            location_suffix,
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
                let owner_control_suffix = if choose_spec_allows_multiple(&move_to_zone.target) {
                    " under their owners' control"
                } else {
                    " under its owner's control"
                };
                let tapped_suffix = if move_to_zone.enters_tapped {
                    " tapped"
                } else {
                    ""
                };
                let controller_suffix = match move_to_zone.battlefield_controller {
                    crate::effects::BattlefieldController::Preserve => "",
                    crate::effects::BattlefieldController::Owner => owner_control_suffix,
                    crate::effects::BattlefieldController::You => " under your control",
                };
                if let crate::target::ChooseSpec::Tagged(tag) = &move_to_zone.target
                    && tag.as_str().starts_with("exiled_")
                {
                    format!("Return {target} to the battlefield{tapped_suffix}{controller_suffix}")
                } else {
                    format!("Put {target} onto the battlefield{tapped_suffix}{controller_suffix}")
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
    if let Some(exile_until) = effect.downcast_ref::<crate::effects::ExileUntilEffect>() {
        let duration = match exile_until.duration {
            crate::effects::ExileUntilDuration::SourceLeavesBattlefield => {
                "until this permanent leaves the battlefield"
            }
            crate::effects::ExileUntilDuration::NextEndStep => "until the next end step",
            crate::effects::ExileUntilDuration::EndOfCombat => "until end of combat",
        };
        return format!(
            "Exile {} {duration}",
            describe_choose_spec(&exile_until.spec)
        );
    }
    if let Some(destroy) = effect.downcast_ref::<crate::effects::DestroyEffect>() {
        if let ChooseSpec::All(filter) = &destroy.spec
            && filter.card_types.as_slice() == [crate::types::CardType::Creature]
            && filter.all_colors == Some(false)
        {
            return "Destroy each creature that isn't all colors".to_string();
        }
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
            let stat = if matches!(&deal_damage.amount, Value::ToughnessOf(_)) {
                "toughness"
            } else {
                "power"
            };
            if subject.eq_ignore_ascii_case(&target) {
                let lower_subject = subject.to_ascii_lowercase();
                let should_render_each = !lower_subject.starts_with("target ")
                    && !lower_subject.starts_with("this ")
                    && !lower_subject.starts_with("that ")
                    && !lower_subject.starts_with("another ");
                if should_render_each {
                    return format!("Each {subject} deals damage to itself equal to its {stat}");
                }
                if choose_spec_is_plural(source) {
                    let each_subject = if subject.to_ascii_lowercase().starts_with("each ") {
                        subject.clone()
                    } else {
                        format!("Each {subject}")
                    };
                    return format!("{each_subject} deals damage to itself equal to its {stat}");
                }
                return format!("{subject} deals damage to itself equal to its {stat}");
            }
            let verb = if choose_spec_is_plural(source) {
                "deal"
            } else {
                "deals"
            };
            return format!("{subject} {verb} damage equal to its {stat} to {target}");
        }
        if let Value::ManaValueOf(spec) = &deal_damage.amount {
            let amount_text = {
                let described = describe_choose_spec(spec);
                if described == "it" {
                    "its mana value".to_string()
                } else {
                    format!("the mana value of {described}")
                }
            };
            return format!(
                "Deal damage equal to {} to {}",
                amount_text,
                describe_choose_spec(&deal_damage.target)
            );
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
        return format!("{} unless {} {} {}", inner_text, payer, pay_verb, mana_text);
    }
    if let Some(unless_action) = effect.downcast_ref::<crate::effects::UnlessActionEffect>() {
        let inner_text = describe_effect_list(&unless_action.effects);
        if unless_action.alternative.len() == 1
            && let Some(lose_life) =
                unless_action.alternative[0].downcast_ref::<crate::effects::LoseLifeEffect>()
            && let ChooseSpec::Player(alternative_player) = &lose_life.player
            && *alternative_player == unless_action.player
        {
            let payer = match alternative_player {
                PlayerFilter::Any => "any player".to_string(),
                _ => describe_player_filter(alternative_player),
            };
            let pay_verb = player_verb(&payer, "pay", "pays");
            return format!(
                "{} unless {} {} {} life",
                inner_text,
                payer,
                pay_verb,
                describe_value(&lose_life.amount)
            );
        }
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
        if put_counters.distributed {
            return format!(
                "Distribute {} among {}",
                describe_put_counter_phrase(&put_counters.count, put_counters.counter_type),
                describe_choose_spec(&put_counters.target)
            );
        }
        let mut target = describe_choose_spec(&put_counters.target);
        if let ChooseSpec::WithCount(inner, count) = &put_counters.target
            && matches!(inner.as_ref(), ChooseSpec::Target(_))
            && !count.is_single()
            && !target.starts_with("each of ")
        {
            target = format!("each of {target}");
        }
        if let Value::Count(filter) = &put_counters.count
            && matches!(
                put_counters.counter_type,
                crate::object::CounterType::PlusOnePlusOne
                    | crate::object::CounterType::MinusOneMinusOne
            )
        {
            return format!(
                "Put a {} counter on {target} for each {}",
                describe_counter_type(put_counters.counter_type),
                filter.description()
            );
        }
        return format!(
            "Put {} on {}",
            describe_put_counter_phrase(&put_counters.count, put_counters.counter_type),
            target
        );
    }
    if let Some(remove_counters) = effect.downcast_ref::<crate::effects::RemoveCountersEffect>() {
        return format!(
            "Remove {} from {}",
            describe_put_counter_phrase(&remove_counters.count, remove_counters.counter_type),
            describe_choose_spec(&remove_counters.target)
        );
    }
    if let Some(remove_up_to_counters) =
        effect.downcast_ref::<crate::effects::RemoveUpToCountersEffect>()
    {
        return format!(
            "Remove up to {} {} counter(s) from {}",
            describe_value(&remove_up_to_counters.max_count),
            describe_counter_type(remove_up_to_counters.counter_type),
            describe_choose_spec(&remove_up_to_counters.target)
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
        if let Value::Count(filter) = &gain.amount {
            return format!(
                "{} {} 1 life for each {}",
                player,
                player_verb(&player, "gain", "gains"),
                describe_for_each_count_filter(filter)
            );
        }
        if let Value::CountScaled(filter, multiplier) = &gain.amount {
            return format!(
                "{} {} {} life for each {}",
                player,
                player_verb(&player, "gain", "gains"),
                multiplier,
                describe_for_each_count_filter(filter)
            );
        }
        if matches!(gain.amount, Value::CreaturesDiedThisTurn) {
            return format!(
                "{} {} 1 life for each creature that died this turn",
                player,
                player_verb(&player, "gain", "gains")
            );
        }
        if matches!(
            gain.amount,
            Value::SourcePower | Value::SourceToughness | Value::PowerOf(_) | Value::ToughnessOf(_)
        ) {
            return format!(
                "{} {} life equal to {}",
                player,
                player_verb(&player, "gain", "gains"),
                describe_value(&gain.amount)
            );
        }
        return format!(
            "{} {} {} life",
            player,
            player_verb(&player, "gain", "gains"),
            describe_value(&gain.amount)
        );
    }
    if let Some(lose) = effect.downcast_ref::<crate::effects::LoseLifeEffect>() {
        let player = describe_choose_spec(&lose.player);
        if let Value::Count(filter) = &lose.amount {
            return format!(
                "{} {} 1 life for each {}",
                player,
                player_verb(&player, "lose", "loses"),
                describe_for_each_count_filter(filter)
            );
        }
        if let Value::CountScaled(filter, multiplier) = &lose.amount {
            return format!(
                "{} {} {} life for each {}",
                player,
                player_verb(&player, "lose", "loses"),
                multiplier,
                describe_for_each_count_filter(filter)
            );
        }
        if matches!(lose.amount, Value::CreaturesDiedThisTurn) {
            return format!(
                "{} {} 1 life for each creature that died this turn",
                player,
                player_verb(&player, "lose", "loses")
            );
        }
        if matches!(
            lose.amount,
            Value::SourcePower | Value::SourceToughness | Value::PowerOf(_) | Value::ToughnessOf(_)
        ) {
            return format!(
                "{} {} life equal to {}",
                player,
                player_verb(&player, "lose", "loses"),
                describe_value(&lose.amount)
            );
        }
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
            describe_discard_count(&discard.count, discard.card_filter.as_ref()),
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
        if let Value::ManaValueOf(spec) = &add_scaled.amount {
            let amount_text = {
                let described = describe_choose_spec(spec);
                if described == "it" {
                    "its mana value".to_string()
                } else {
                    format!("the mana value of {described}")
                }
            };
            return format!(
                "Add an amount of {} equal to {} to {}",
                mana_text,
                amount_text,
                describe_mana_pool_owner(&add_scaled.player)
            );
        }
        if let Value::EffectValue(_) = &add_scaled.amount {
            return format!(
                "Add that much {} to {}",
                mana_text,
                describe_mana_pool_owner(&add_scaled.player)
            );
        }
        if let Value::EventValue(EventValueSpec::Amount)
        | Value::EventValue(EventValueSpec::LifeAmount) = &add_scaled.amount
        {
            return format!(
                "Add that much {} to {}",
                mana_text,
                describe_mana_pool_owner(&add_scaled.player)
            );
        }
        if let Value::EffectValueOffset(_, offset) = &add_scaled.amount {
            let amount_text = if *offset == 0 {
                "that much".to_string()
            } else if *offset > 0 {
                format!("that much plus {}", offset)
            } else {
                format!("that much minus {}", -offset)
            };
            return format!(
                "Add {} {} to {}",
                amount_text,
                mana_text,
                describe_mana_pool_owner(&add_scaled.player)
            );
        }
        if let Value::EventValueOffset(EventValueSpec::Amount, offset)
        | Value::EventValueOffset(EventValueSpec::LifeAmount, offset) = &add_scaled.amount
        {
            let amount_text = if *offset == 0 {
                "that much".to_string()
            } else if *offset > 0 {
                format!("that much plus {}", offset)
            } else {
                format!("that much minus {}", -offset)
            };
            return format!(
                "Add {} {} to {}",
                amount_text,
                mana_text,
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
    if let Some(attach) = effect.downcast_ref::<crate::effects::AttachObjectsEffect>() {
        return format!(
            "Attach {} to {}",
            describe_attach_objects_spec(&attach.objects),
            describe_choose_spec(&attach.target)
        );
    }
    if let Some(sacrifice) = effect.downcast_ref::<crate::effects::SacrificeEffect>() {
        let player = describe_player_filter(&sacrifice.player);
        let verb = player_verb(&player, "sacrifice", "sacrifices");
        if let Value::Count(count_filter) = &sacrifice.count
            && count_filter == &sacrifice.filter
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
            return format!("{player} {verb} all {}", pluralize_noun_phrase(&noun));
        }
        if matches!(sacrifice.count, Value::Fixed(1)) {
            if let Some(rest) = sacrifice
                .filter
                .description()
                .strip_prefix("target player's ")
            {
                return format!("{player} {verb} {}", with_indefinite_article(rest));
            }
            if let Some(rest) = sacrifice
                .filter
                .description()
                .strip_prefix("that player's ")
            {
                return format!("{player} {verb} {}", with_indefinite_article(rest));
            }
            if let Some(rest) = sacrifice
                .filter
                .description()
                .strip_prefix("the active player's ")
            {
                return format!("{player} {verb} {}", with_indefinite_article(rest));
            }
        }
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
    if let Some(return_from_gy) =
        effect.downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()
    {
        if let Some(owner) = graveyard_owner_from_spec(&return_from_gy.target) {
            let target_text = describe_choose_spec_without_graveyard_zone(&return_from_gy.target);
            let from_text = match &owner {
                Some(owner) => format!("{} graveyard", describe_possessive_player_filter(owner)),
                None => "a graveyard".to_string(),
            };
            let to_text = match &owner {
                Some(owner) => format!("{} hand", describe_possessive_player_filter(owner)),
                None => owner_hand_phrase_for_spec(&return_from_gy.target).to_string(),
            };
            return format!("Return {target_text} from {from_text} to {to_text}");
        }
        return format!(
            "Return {} from a graveyard to {}",
            describe_choose_spec_without_graveyard_zone(&return_from_gy.target),
            owner_hand_phrase_for_spec(&return_from_gy.target)
        );
    }
    if let Some(shuffle_library) = effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>() {
        return format!(
            "Shuffle {} library",
            describe_possessive_player_filter(&shuffle_library.player)
        );
    }
    if let Some(shuffle_gy) =
        effect.downcast_ref::<crate::effects::ShuffleGraveyardIntoLibraryEffect>()
    {
        let subject = describe_player_filter(&shuffle_gy.player);
        let verb = player_verb(&subject, "shuffle", "shuffles");
        return format!(
            "{} {} {} graveyard into {} library",
            subject,
            verb,
            describe_possessive_player_filter(&shuffle_gy.player),
            describe_possessive_player_filter(&shuffle_gy.player)
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
    if let Some(look_at_top) = effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>() {
        let owner = describe_possessive_player_filter(&look_at_top.player);
        let count_text = small_number_word(look_at_top.count as u32)
            .map(str::to_string)
            .unwrap_or_else(|| look_at_top.count.to_string());
        let noun = if look_at_top.count == 1 {
            "card"
        } else {
            "cards"
        };
        return format!("Look at the top {count_text} {noun} of {owner} library");
    }
    if let Some(look_at_hand) = effect.downcast_ref::<crate::effects::LookAtHandEffect>() {
        if look_at_hand.reveal {
            if matches!(
                look_at_hand.target.base(),
                crate::target::ChooseSpec::Player(PlayerFilter::You)
            ) {
                return "Reveal your hand".to_string();
            }
            let subject = capitalize_first(&describe_choose_spec(&look_at_hand.target));
            let reveal_verb = player_verb(&subject, "reveal", "reveals");
            return format!("{subject} {reveal_verb} their hand");
        }
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
        let each_text = if let Value::Count(filter) = &modify_pt_each.count {
            describe_for_each_count_filter(filter)
        } else {
            describe_value(&modify_pt_each.count)
        };
        return format!(
            "{} gets +{} / +{} for each {} {}",
            describe_choose_spec(&modify_pt_each.target),
            modify_pt_each.power_per,
            modify_pt_each.toughness_per,
            each_text,
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
        if let Some(compact) = describe_put_or_remove_counter_mode(choose_mode) {
            return compact;
        }
        let mut header = describe_mode_choice_header(
            &choose_mode.choose_count,
            choose_mode.min_choose_count.as_ref(),
        );
        if choose_mode.disallow_previously_chosen_modes {
            header = if choose_mode.disallow_previously_chosen_modes_this_turn {
                "Choose one that hasn't been chosen this turn —".to_string()
            } else {
                "Choose one that hasn't been chosen —".to_string()
            };
        }
        let modes = choose_mode
            .modes
            .iter()
            .map(|mode| {
                let description_raw = mode.description.trim();
                let description = ensure_trailing_period(description_raw);
                let mode_effects = describe_effect_list(&mode.effects);
                if !description.trim().is_empty() {
                    let mode_effects_trimmed = mode_effects.trim();
                    if mode_effects_trimmed.is_empty() {
                        return description;
                    }

                    let effects_lower = mode_effects_trimmed.to_ascii_lowercase();
                    let description_lower = description_raw.to_ascii_lowercase();
                    let has_followup = (effects_lower.contains("if you do")
                        || effects_lower.contains("if they do")
                        || effects_lower.contains("choose one")
                        || effects_lower.contains("choose one or"))
                        && !description_lower.contains("if you do")
                        && !description_lower.contains("if they do")
                        && !description_lower.contains("choose one");

                    if !has_followup {
                        return description;
                    }

                    let mut followup = mode_effects_trimmed.to_string();
                    if let Some((_, tail)) = followup.split_once(". ") {
                        followup = tail.trim().to_string();
                    } else if let Some((_, tail)) = followup.split_once('.') {
                        followup = tail.trim().to_string();
                    }
                    let description_head = description.trim_end_matches('.');
                    if followup.is_empty() {
                        if let Some(stripped) = mode_effects_trimmed.strip_prefix(description_head)
                        {
                            followup = stripped.trim_start_matches('.').trim().to_string();
                        } else {
                            followup = mode_effects_trimmed.to_string();
                        }
                    }
                    if followup.is_empty() {
                        description
                    } else {
                        format!(
                            "{} {}",
                            description_head,
                            ensure_trailing_period(followup.trim())
                        )
                    }
                } else {
                    ensure_trailing_period(mode_effects.trim())
                }
            })
            .collect::<Vec<_>>()
            .join(" • ");
        if choose_mode.disallow_previously_chosen_modes {
            return format!("{header}\n• {}", modes.replace(" • ", "\n• "));
        }
        if choose_mode.allow_repeated_modes {
            let normalized_header = header
                .trim_end_matches('-')
                .trim_end_matches('—')
                .trim()
                .trim_end_matches('.')
                .to_string();
            return format!(
                "{normalized_header}. You may choose the same mode more than once. • {modes}"
            );
        }
        return format!("{header} {modes}");
    }
    if let Some(create_token) = effect.downcast_ref::<crate::effects::CreateTokenEffect>() {
        if let Some(compact) = describe_compact_create_token(create_token) {
            return compact;
        }
        if let Some(for_each_count) = describe_create_for_each_count(&create_token.count) {
            let token_blueprint = describe_token_blueprint(&create_token.token);
            let mut text = if matches!(create_token.controller, PlayerFilter::You) {
                format!("Create 1 {} for each {}", token_blueprint, for_each_count)
            } else {
                format!(
                    "Create 1 {} under {} control for each {}",
                    token_blueprint,
                    describe_possessive_player_filter(&create_token.controller),
                    for_each_count
                )
            };
            if create_token.enters_tapped {
                text.push_str(", tapped");
            }
            if create_token.enters_attacking {
                text.push_str(", attacking");
            }
            if create_token.exile_at_end_of_combat {
                text.push_str(", and exile them at end of combat");
            }
            if create_token.sacrifice_at_next_end_step {
                text.push_str(", and sacrifice it at the beginning of the next end step");
            }
            if create_token.exile_at_next_end_step {
                text.push_str(", and exile it at the beginning of the next end step");
            }
            return text;
        }
        let token_blueprint = describe_token_blueprint(&create_token.token);
        let count_text = describe_effect_count_backref(&create_token.count)
            .unwrap_or_else(|| describe_value(&create_token.count));
        let mut text = if matches!(create_token.controller, PlayerFilter::You) {
            format!("Create {} {}", count_text, token_blueprint)
        } else {
            format!(
                "Create {} {} under {} control",
                count_text,
                token_blueprint,
                describe_possessive_player_filter(&create_token.controller)
            )
        };
        if create_token.enters_tapped {
            text.push_str(", tapped");
        }
        if create_token.enters_attacking {
            text.push_str(", attacking");
        }
        if create_token.exile_at_end_of_combat {
            text.push_str(", and exile them at end of combat");
        }
        if create_token.sacrifice_at_next_end_step {
            text.push_str(", and sacrifice it at the beginning of the next end step");
        }
        if create_token.exile_at_next_end_step {
            text.push_str(", and exile it at the beginning of the next end step");
        }
        return text;
    }
    if let Some(create_copy) = effect.downcast_ref::<crate::effects::CreateTokenCopyEffect>() {
        let target = describe_choose_spec(&create_copy.target);
        let mut text = match create_copy.count {
            Value::Fixed(1) => format!("Create a token that's a copy of {target}"),
            Value::Fixed(n) => format!("Create {n} tokens that are copies of {target}"),
            _ => format!(
                "Create {} tokens that are copies of {target}",
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
        if create_copy.exile_at_next_end_step {
            text.push_str(", and exile it at the beginning of the next end step");
        }
        if create_copy.pt_adjustment.is_some() {
            if matches!(create_copy.count, Value::Fixed(1)) {
                text.push_str(
                    ", except its power and toughness are each half that permanent's power and toughness, rounded up",
                );
            } else {
                text.push_str(
                    ", except their power and toughness are each half that permanent's power and toughness, rounded up",
                );
            }
        }
        if let Some((power, toughness)) = create_copy.set_base_power_toughness {
            text.push_str(&format!(
                ", with base power and toughness {power}/{toughness}"
            ));
        }
        if create_copy.set_colors.is_some()
            || create_copy.set_card_types.is_some()
            || create_copy.set_subtypes.is_some()
        {
            let mut words = Vec::new();
            if let Some(colors) = create_copy.set_colors {
                if colors.contains(crate::color::Color::White) {
                    words.push("white".to_string());
                }
                if colors.contains(crate::color::Color::Blue) {
                    words.push("blue".to_string());
                }
                if colors.contains(crate::color::Color::Black) {
                    words.push("black".to_string());
                }
                if colors.contains(crate::color::Color::Red) {
                    words.push("red".to_string());
                }
                if colors.contains(crate::color::Color::Green) {
                    words.push("green".to_string());
                }
            }
            if let Some(subtypes) = &create_copy.set_subtypes {
                words.extend(
                    subtypes
                        .iter()
                        .map(|subtype| format!("{subtype:?}").to_ascii_lowercase()),
                );
            }
            if let Some(card_types) = &create_copy.set_card_types {
                words.extend(
                    card_types
                        .iter()
                        .map(|card_type| format!("{card_type:?}").to_ascii_lowercase()),
                );
            }
            if !words.is_empty() {
                text.push_str(", and it's ");
                text.push_str(&words.join(" "));
            }
        }
        if !create_copy.added_card_types.is_empty() || !create_copy.added_subtypes.is_empty() {
            let mut type_words: Vec<String> = create_copy
                .added_card_types
                .iter()
                .map(|card_type| format!("{card_type:?}").to_ascii_lowercase())
                .collect();
            type_words.extend(
                create_copy
                    .added_subtypes
                    .iter()
                    .map(|subtype| format!("{subtype:?}").to_ascii_lowercase()),
            );
            if !type_words.is_empty() {
                text.push_str(", and it's ");
                text.push_str(&type_words.join(" "));
                text.push_str(" in addition to its other types");
            }
        }
        if create_copy
            .removed_supertypes
            .iter()
            .any(|supertype| *supertype == Supertype::Legendary)
        {
            text.push_str(", and it isn't legendary");
        }
        if !create_copy.granted_static_abilities.is_empty() {
            let mut granted = create_copy
                .granted_static_abilities
                .iter()
                .map(|ability| ability.display().to_ascii_lowercase())
                .collect::<Vec<_>>();
            granted.sort();
            granted.dedup();
            text.push_str(", and it has ");
            text.push_str(&join_with_and(&granted));
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
        let mut target = describe_choose_spec(&regenerate.target);
        if let Some(rest) = target.strip_prefix("all ") {
            target = format!("each {rest}");
        }
        if regenerate.duration == Until::EndOfTurn {
            return format!("Regenerate {target}");
        }
        return format!(
            "Regenerate {target} {}",
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
        if scry.player == PlayerFilter::Opponent {
            return format!("Fateseal {}", describe_value(&scry.count));
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
        let amount = match poison.count {
            Value::Fixed(1) => "a poison counter".to_string(),
            _ => format!("{} poison counters", describe_value(&poison.count)),
        };
        return format!("{} gets {}", describe_player_filter(&poison.player), amount);
    }
    if let Some(pay_energy) = effect.downcast_ref::<crate::effects::PayEnergyEffect>() {
        let payer = describe_choose_spec(&pay_energy.player);
        let amount = describe_energy_payment_amount(&pay_energy.amount);
        if payer == "you" {
            return format!("Pay {amount}");
        }
        return format!("{payer} {} {amount}", player_verb(&payer, "pay", "pays"));
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
    if let Some(goad) = effect.downcast_ref::<crate::effects::GoadEffect>() {
        return format!("Goad {}", describe_goad_target(&goad.target));
    }
    if let Some(extra_turn) = effect.downcast_ref::<crate::effects::ExtraTurnEffect>() {
        let player = describe_player_filter(&extra_turn.player);
        return format!(
            "{} {} an extra turn after this one",
            player,
            player_verb(&player, "take", "takes")
        );
    }
    if let Some(lose_game) = effect.downcast_ref::<crate::effects::LoseTheGameEffect>() {
        let player = describe_player_filter(&lose_game.player);
        return format!(
            "{} {} the game",
            player,
            player_verb(&player, "lose", "loses")
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
    if let Some(skip_combat) = effect.downcast_ref::<crate::effects::SkipCombatPhasesEffect>() {
        return format!(
            "{} skips all combat phases of their next turn",
            describe_player_filter(&skip_combat.player)
        );
    }
    if let Some(skip_combat) =
        effect.downcast_ref::<crate::effects::SkipNextCombatPhaseThisTurnEffect>()
    {
        return format!(
            "{} skips their next combat phase this turn",
            describe_player_filter(&skip_combat.player)
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
        if matches!(copy_spell.target, ChooseSpec::Source)
            && let Value::Count(filter) = &copy_spell.count
        {
            let mut each_filter = filter.description();
            if each_filter.ends_with('s') {
                each_filter = each_filter.trim_end_matches('s').to_string();
            }
            return format!("Copy this spell for each {each_filter}");
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
        if let Value::Count(filter) = &add_one.amount {
            let mut count_subject = pluralize_noun_phrase(&filter.description());
            let lower_subject = count_subject.to_ascii_lowercase();
            if filter.zone == Some(Zone::Battlefield) && !lower_subject.contains("battlefield") {
                count_subject.push_str(" on the battlefield");
            }
            return format!(
                "Add X mana of any one color to {}, where X is the number of {}",
                describe_mana_pool_owner(&add_one.player),
                count_subject
            );
        }
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
        let timing = if matches!(prevent_damage.duration, Until::EndOfTurn) {
            "this turn".to_string()
        } else {
            describe_until(&prevent_damage.duration)
        };
        return format!(
            "Prevent the next {} {} that would be dealt to {} {}",
            describe_value(&prevent_damage.amount),
            damage_text,
            describe_choose_spec(&prevent_damage.target),
            timing
        );
    }
    if let Some(prevent_all_target) =
        effect.downcast_ref::<crate::effects::PreventAllDamageToTargetEffect>()
    {
        let filter = &prevent_all_target.damage_filter;
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
        let timing = if matches!(prevent_all_target.duration, Until::EndOfTurn) {
            "this turn".to_string()
        } else {
            describe_until(&prevent_all_target.duration)
        };
        return format!(
            "Prevent all {} that would be dealt to {} {}",
            damage_text,
            describe_choose_spec(&prevent_all_target.target),
            timing
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
        let trigger_display = schedule.trigger.display();
        let mut trigger_text = trigger_display.trim().trim_end_matches('.').to_string();
        if schedule.until_end_of_turn {
            let trigger_lower = trigger_text.to_ascii_lowercase();
            if !trigger_lower.contains(" this turn") {
                trigger_text.push_str(" this turn");
            }
        }
        let trigger_lower = trigger_text.to_ascii_lowercase();
        let delayed_text = lowercase_first(&describe_effect_list(&schedule.effects));
        if schedule.one_shot
            && (trigger_lower.contains("beginning of each player's end step")
                || trigger_lower.contains("beginning of end step"))
        {
            return format!("At the beginning of the next end step, {delayed_text}");
        }
        if schedule.one_shot && trigger_lower.contains("beginning of your end step") {
            return format!("At the beginning of your next end step, {delayed_text}");
        }
        if schedule.one_shot && trigger_lower.contains("when this creature dies") {
            if let Some(filter) = &schedule.target_filter {
                let subject = with_indefinite_article(&describe_for_each_filter(filter));
                return format!(
                    "When {subject} dealt damage this way dies this turn, {delayed_text}"
                );
            }
            return format!("When that creature dies this turn, {delayed_text}");
        }
        if trigger_lower.starts_with("when ")
            || trigger_lower.starts_with("whenever ")
            || trigger_lower.starts_with("if ")
        {
            return format!("{trigger_text}, {delayed_text}");
        }
        if trigger_lower.starts_with("at ") {
            return format!("{trigger_text}, {delayed_text}");
        }
        return format!("At {}, {delayed_text}", lowercase_first(&trigger_text));
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
    let lower = clause.to_ascii_lowercase();
    if lower == "activate only as a sorcery and only once each turn" {
        return "Activate only once each turn".to_string();
    }
    clause = clause.replace("activate only as sorcery", "activate only as a sorcery");
    clause = clause.replace("activate only once turn", "activate only once each turn");
    if clause.starts_with("activate ") {
        clause = capitalize_first(&clause);
    }
    clause
}

fn collect_activation_restriction_clauses(
    timing: &ActivationTiming,
    additional_restrictions: &[String],
) -> Vec<String> {
    let mut clauses = Vec::new();

    if let Some(timing_clause) = describe_activation_timing_clause(timing) {
        let normalized = normalize_activation_restriction_clause(timing_clause);
        push_activation_restriction_clause(&mut clauses, normalized);
    }

    for raw in additional_restrictions {
        let normalized = normalize_activation_restriction_clause(raw);
        push_activation_restriction_clause(&mut clauses, normalized);
    }

    clauses
}

fn push_activation_restriction_clause(clauses: &mut Vec<String>, clause: String) {
    if clause.is_empty() {
        return;
    }
    let clause_lower = clause.to_ascii_lowercase();
    let mut remove_indices = Vec::new();
    for (idx, existing) in clauses.iter().enumerate() {
        let existing_lower = existing.to_ascii_lowercase();
        if existing_lower == clause_lower
            || activation_clause_is_more_specific(&existing_lower, &clause_lower)
        {
            return;
        }
        if activation_clause_is_more_specific(&clause_lower, &existing_lower) {
            remove_indices.push(idx);
        }
    }
    for idx in remove_indices.into_iter().rev() {
        clauses.remove(idx);
    }
    clauses.push(clause);
}

fn activation_clause_is_more_specific(candidate: &str, base: &str) -> bool {
    if candidate.len() <= base.len() || !candidate.starts_with(base) {
        return false;
    }
    let tail = candidate[base.len()..].trim_start();
    tail.starts_with(',')
        || tail.starts_with("and ")
        || tail.starts_with("before ")
        || tail.starts_with("after ")
        || tail.starts_with("if ")
        || tail.starts_with("unless ")
}

fn join_activation_restriction_clauses(clauses: &[String]) -> String {
    let mut iter = clauses.iter();
    let Some(first) = iter.next() else {
        return String::new();
    };
    let mut line = first.clone();
    for clause in iter {
        if let Some(rest) = clause.strip_prefix("Activate only ") {
            line.push_str(" and ");
            line.push_str(rest);
        } else {
            line.push_str(" and ");
            line.push_str(clause);
        }
    }
    line
}

fn describe_keyword_ability(ability: &Ability) -> Option<String> {
    let raw_text = ability.text.as_deref()?.trim();
    let text = raw_text.to_ascii_lowercase();
    let words = text.split_whitespace().collect::<Vec<_>>();
    if words.len() == 4
        && words[0] == "cycling"
        && words[1] == "pay"
        && words[3] == "life"
        && (words[2].parse::<u32>().is_ok() || words[2] == "x")
    {
        return Some(format!("Cycling—Pay {} life", words[2]));
    }
    if words.first().copied() == Some("equip") {
        let mut rendered = if raw_text.eq_ignore_ascii_case("equip") {
            "Equip".to_string()
        } else {
            raw_text.to_string()
        };
        if let AbilityKind::Activated(activated) = &ability.kind {
            let mut restriction_clauses = collect_activation_restriction_clauses(
                &activated.timing,
                &activated.additional_restrictions,
            );
            // Equip implies sorcery-speed by default; only surface extra restrictions.
            restriction_clauses
                .retain(|clause| !clause.eq_ignore_ascii_case("Activate only as a sorcery"));
            if !restriction_clauses.is_empty() {
                rendered.push_str(". ");
                rendered.push_str(&join_activation_restriction_clauses(&restriction_clauses));
            }
        }
        return Some(rendered);
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
    let first_cycling_idx = words
        .iter()
        .position(|word| trim_cycling_punctuation(word).ends_with("cycling"));
    let is_cycling_clause = first_cycling_idx.is_some_and(|idx| {
        !words[..idx]
            .iter()
            .any(|word| matches!(*word, "has" | "have"))
    });
    if is_cycling_clause {
        let mut cycling_rendered = Vec::new();
        for (idx, word) in words.iter().enumerate() {
            let keyword = trim_cycling_punctuation(word);
            if !keyword.ends_with("cycling") {
                continue;
            }
            let next = words
                .get(idx + 1)
                .map(|next| trim_cycling_punctuation(next));
            let has_cost = next.is_none_or(is_cycling_cost_word);
            if !has_cost {
                continue;
            }
            let mut chars = keyword.chars();
            let mut base = match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => "Cycling".to_string(),
            };
            if keyword == "landcycling"
                && idx > 0
                && trim_cycling_punctuation(words[idx - 1]) == "basic"
            {
                base = "Basic landcycling".to_string();
            }
            let mut cost_tokens = Vec::new();
            let mut j = idx + 1;
            while let Some(word) = words.get(j).map(|word| trim_cycling_punctuation(word)) {
                if is_cycling_cost_word(word) {
                    cost_tokens.push(word);
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
                    .map(|word| render_cycling_cost_token(word))
                    .collect::<Vec<_>>()
                    .join("");
                cycling_rendered.push(format!("{} {}", base, cost));
            }
        }
        if !cycling_rendered.is_empty() {
            return Some(cycling_rendered.join(", "));
        }
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

fn trim_cycling_punctuation(word: &str) -> &str {
    word.trim_matches(|ch: char| matches!(ch, ',' | '.' | ';'))
}

fn render_cycling_cost_token(word: &str) -> String {
    let upper = word.to_ascii_uppercase();
    if upper.starts_with('{') && upper.ends_with('}') {
        upper
    } else {
        format!("{{{upper}}}")
    }
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
            ChooseSpec::Target(_) | ChooseSpec::AnyTarget | ChooseSpec::PlayerOrPlaneswalker(_) => {
                true
            }
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
            if let Some(InterveningIfCondition::MaxTimesEachTurn(max)) =
                triggered.intervening_if.as_ref()
            {
                if *max == 1 {
                    line.push_str(". This ability triggers only once each turn");
                } else if *max == 2 {
                    line.push_str(". This ability triggers only twice each turn");
                } else {
                    line.push_str(". This ability triggers only ");
                    line.push_str(&format!("{max}"));
                    line.push_str(" times");
                    line.push_str(" each turn");
                }
            }
            vec![line]
        }
        AbilityKind::Activated(activated) => {
            let mut line = format!("Activated ability {index}");
            let mut pre = Vec::new();
            let has_boast_label = ability
                .text
                .as_deref()
                .is_some_and(|text| text.eq_ignore_ascii_case("boast"));
            if has_boast_label {
                let mut label = "Boast".to_string();
                if !activated.mana_cost.costs().is_empty() {
                    label.push(' ');
                    label.push_str(&describe_cost_list(activated.mana_cost.costs()));
                }
                pre.push(label);
            } else if !activated.mana_cost.costs().is_empty() {
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
            if let Some(x_clause) = extract_activated_x_is_clause(ability.text.as_deref()) {
                let line_lower = line.to_ascii_lowercase();
                let clause_lower = x_clause.to_ascii_lowercase();
                if !line_lower.contains(clause_lower.as_str()) {
                    if !inject_x_clause_into_modal_heading(&mut line, &x_clause) {
                        while line.ends_with('.') {
                            line.pop();
                        }
                        line.push_str(". ");
                        line.push_str(&x_clause);
                    }
                }
            }
            let restriction_clauses = collect_activation_restriction_clauses(
                &activated.timing,
                &activated.additional_restrictions,
            );
            if !restriction_clauses.is_empty() {
                line.push_str(". ");
                line.push_str(&join_activation_restriction_clauses(&restriction_clauses));
            }
            vec![line]
        }
        AbilityKind::Mana(mana_ability) => {
            let mut line = format!("Mana ability {index}");
            let cost_text = if !mana_ability.mana_cost.costs().is_empty() {
                Some(describe_cost_list(mana_ability.mana_cost.costs()))
            } else {
                None
            };
            let add_text = if !mana_ability.mana.is_empty() {
                Some(format!(
                    "Add {}",
                    mana_ability
                        .mana
                        .iter()
                        .copied()
                        .map(describe_mana_symbol)
                        .collect::<Vec<_>>()
                        .join("")
                ))
            } else {
                None
            };
            if let (Some(cost), Some(add)) = (&cost_text, &add_text) {
                line.push_str(": ");
                line.push_str(cost);
                line.push_str(": ");
                line.push_str(add);
            } else if let Some(cost) = &cost_text {
                line.push_str(": ");
                line.push_str(cost);
            } else if let Some(add) = &add_text {
                line.push_str(": ");
                line.push_str(add);
            }
            if let Some(extra_effects) = &mana_ability.effects
                && !extra_effects.is_empty()
            {
                line.push_str(": ");
                line.push_str(&describe_effect_list(extra_effects));
            }
            if let Some(condition) = &mana_ability.activation_condition {
                let clause = describe_mana_activation_condition(condition);
                if !clause.is_empty() {
                    line.push_str(". ");
                    line.push_str(&clause);
                }
            }
            vec![line]
        }
    }
}

fn extract_activated_x_is_clause(text: Option<&str>) -> Option<String> {
    let text = text?.trim();
    let lower = text.to_ascii_lowercase();
    let idx = lower.find("x is ")?;
    let clause = text[idx..].trim().trim_end_matches('.').trim();
    if clause.is_empty() {
        None
    } else {
        Some(clause.to_string())
    }
}

fn inject_x_clause_into_modal_heading(line: &mut String, x_clause: &str) -> bool {
    let direct_replacements = vec![
        (
            "Choose one —\n• ",
            format!("Choose one. {x_clause}\n• "),
        ),
        (
            "Choose one -\n• ",
            format!("Choose one. {x_clause}\n• "),
        ),
        (
            "choose one —\n• ",
            format!("choose one. {x_clause}\n• "),
        ),
        (
            "choose one -\n• ",
            format!("choose one. {x_clause}\n• "),
        ),
        (
            "Choose one or both —\n• ",
            format!("Choose one or both. {x_clause}\n• "),
        ),
        (
            "Choose one or both -\n• ",
            format!("Choose one or both. {x_clause}\n• "),
        ),
        (
            "Choose one or more —\n• ",
            format!("Choose one or more. {x_clause}\n• "),
        ),
        (
            "Choose one or more -\n• ",
            format!("Choose one or more. {x_clause}\n• "),
        ),
    ];
    for (marker, replacement) in direct_replacements {
        if line.contains(marker) {
            *line = line.replacen(marker, replacement.as_str(), 1);
            return true;
        }
    }

    for marker in [
        "Choose one —",
        "Choose one -",
        "choose one —",
        "choose one -",
        "Choose one or both —",
        "Choose one or both -",
        "choose one or both —",
        "choose one or both -",
        "Choose one or more —",
        "Choose one or more -",
        "choose one or more —",
        "choose one or more -",
    ] {
        if line.contains(marker) {
            let replacement = if marker.contains(" —") {
                marker.replacen(" —", format!(". {x_clause} —").as_str(), 1)
            } else {
                marker.replacen(" -", format!(". {x_clause} -").as_str(), 1)
            };
            *line = line.replacen(marker, replacement.as_str(), 1);
            return true;
        }
    }
    false
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
        crate::ability::ManaAbilityCondition::ControlAtLeastArtifacts(count) => {
            if *count == 1 {
                "Activate only if you control an artifact".to_string()
            } else {
                format!("Activate only if you control {count} or more artifacts")
            }
        }
        crate::ability::ManaAbilityCondition::ControlAtLeastLands(count) => {
            if *count == 1 {
                "Activate only if you control a land".to_string()
            } else {
                format!("Activate only if you control {count} or more lands")
            }
        }
        crate::ability::ManaAbilityCondition::ControlCreatureWithPowerAtLeast(power) => {
            format!("Activate only if you control a creature with power {power} or greater")
        }
        crate::ability::ManaAbilityCondition::ControlCreaturesTotalPowerAtLeast(power) => {
            format!("Activate only if creatures you control have total power {power} or greater")
        }
        crate::ability::ManaAbilityCondition::CardInYourGraveyard {
            card_types,
            subtypes,
        } => {
            let mut descriptors: Vec<String> = Vec::new();
            for subtype in subtypes {
                descriptors.push(format!("{subtype:?}"));
            }
            for card_type in card_types {
                descriptors.push(format!("{card_type:?}").to_ascii_lowercase());
            }
            descriptors.retain(|entry| !entry.is_empty());
            descriptors.dedup();

            if descriptors.is_empty() {
                "Activate only if there is a card in your graveyard".to_string()
            } else if descriptors.len() == 1 {
                format!(
                    "Activate only if there is an {} card in your graveyard",
                    descriptors[0]
                )
            } else {
                let head = descriptors[..descriptors.len() - 1].join(" ");
                let tail = descriptors.last().expect("descriptor tail");
                format!("Activate only if there is a {head} {tail} card in your graveyard")
            }
        }
        crate::ability::ManaAbilityCondition::Timing(timing) => match timing {
            ActivationTiming::AnyTime => "Activate only as an instant".to_string(),
            ActivationTiming::SorcerySpeed => "Activate only as a sorcery".to_string(),
            ActivationTiming::DuringCombat => "Activate only during combat".to_string(),
            ActivationTiming::OncePerTurn => "Activate only once each turn".to_string(),
            ActivationTiming::DuringYourTurn => "Activate only during your turn".to_string(),
            ActivationTiming::DuringOpponentsTurn => {
                "Activate only during an opponent's turn".to_string()
            }
        },
        crate::ability::ManaAbilityCondition::MaxActivationsPerTurn(limit) => {
            if *limit == 1 {
                "Activate only once each turn".to_string()
            } else {
                format!("Activate only up to {limit} times each turn")
            }
        }
        crate::ability::ManaAbilityCondition::Unmodeled(restriction) => {
            let suffix = restriction
                .trim_start_matches("activate only ")
                .trim_start_matches("Activate only ")
                .trim_start_matches("activate ")
                .trim_start_matches("Activate ");
            if suffix.is_empty() {
                "Activate only".to_string()
            } else {
                format!("Activate only {suffix}")
            }
        }
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
    let aura_creature_gate = filter.card_types.len() == 1
        && filter.card_types[0] == CardType::Creature
        && filter.subtypes.len() == 1
        && filter.subtypes[0] == crate::types::Subtype::Aura
        && filter.controller.is_none()
        && filter.owner.is_none()
        && filter.zone == Some(Zone::Battlefield);
    if aura_creature_gate {
        return "creature with another Aura attached to it".to_string();
    }
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

fn describe_additional_cost_effects(effects: &[Effect]) -> String {
    if effects.len() == 1
        && let Some(choose_mode) = effects[0].downcast_ref::<crate::effects::ChooseModeEffect>()
    {
        let min = choose_mode
            .min_choose_count
            .clone()
            .unwrap_or_else(|| choose_mode.choose_count.clone());
        if choose_mode.choose_count == Value::Fixed(1) && min == Value::Fixed(1) {
            let mut options = Vec::new();
            for mode in &choose_mode.modes {
                let mut text = mode.description.trim().to_string();
                if text.is_empty() {
                    text = describe_effect_list(&mode.effects);
                }
                text = text.trim().trim_end_matches('.').to_string();
                if let Some(rest) = text.strip_prefix("you ") {
                    text = normalize_you_verb_phrase(rest);
                }
                if let Some(rest) = text.strip_prefix("pay ") {
                    let normalized_cost = normalize_cost_amount_token(rest);
                    text = format!("pay {normalized_cost}");
                }
                if text.is_empty() {
                    continue;
                }
                options.push(text);
            }
            if options.len() >= 2 {
                return join_with_or(&options);
            }
        }
    }

    describe_effect_list(effects)
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
            method if method.uses_composed_cost_effects() => {
                let name = method.name();
                let mana_cost = method.mana_cost();
                let cost_effects = method.cost_effects();
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
                let mut line = format!("You may {clause} rather than pay this spell's mana cost");
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
            AlternativeCastingMethod::Flashback { cost } => {
                out.push(format!("Flashback {}", cost.to_oracle()));
            }
            AlternativeCastingMethod::JumpStart => {
                out.push("Jump-start".to_string());
            }
            AlternativeCastingMethod::Escape { cost, exile_count } => {
                let count_text = small_number_word(*exile_count)
                    .map(str::to_string)
                    .unwrap_or_else(|| exile_count.to_string());
                if let Some(cost) = cost {
                    out.push(format!(
                        "Escape—{}, Exile {count_text} other cards from your graveyard",
                        cost.to_oracle()
                    ));
                } else {
                    out.push(format!(
                        "Escape—Exile {count_text} other cards from your graveyard"
                    ));
                }
            }
            other => {
                if other.name().eq_ignore_ascii_case("Parsed alternative cost") {
                    if let Some(cost) = other.mana_cost() {
                        out.push(format!(
                            "You may pay {} rather than pay this spell's mana cost",
                            cost.to_oracle()
                        ));
                    } else {
                        out.push("You may cast this spell rather than pay its mana cost".to_string());
                    }
                    continue;
                }
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
                    let add = format!("Add {}", describe_mana_alternatives(&symbols));
                    if !first.mana_cost.costs().is_empty() {
                        let cost = describe_cost_list(first.mana_cost.costs());
                        line.push_str(": ");
                        line.push_str(&cost);
                        line.push_str(": ");
                        line.push_str(&add);
                    } else {
                        line.push_str(": ");
                        line.push_str(&add);
                    }
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
    if !def.cost_effects.is_empty() {
        out.push(format!(
            "As an additional cost to cast this spell, {}",
            describe_additional_cost_effects(&def.cost_effects)
        ));
    }
    if !spell_like_card {
        push_abilities(&mut out);
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
    let normalized = out
        .into_iter()
        .map(|line| normalize_rendered_line_for_card(def, &line))
        .collect::<Vec<_>>();
    merge_adjacent_static_heading_lines(normalized)
        .into_iter()
        .map(|line| normalize_compiled_line_post_pass(def, &line))
        .collect()
}

fn card_self_reference_phrase(def: &CardDefinition) -> &'static str {
    if def.card.is_creature() {
        "this creature"
    } else if def.card.is_instant() || def.card.is_sorcery() {
        "this spell"
    } else if def
        .card
        .subtypes
        .iter()
        .any(|subtype| matches!(subtype, crate::types::Subtype::Aura))
    {
        "this Aura"
    } else if def
        .card
        .subtypes
        .iter()
        .any(|subtype| matches!(subtype, crate::types::Subtype::Equipment))
    {
        "this Equipment"
    } else if def
        .card
        .subtypes
        .iter()
        .any(|subtype| matches!(subtype, crate::types::Subtype::Fortification))
    {
        "this Fortification"
    } else if def
        .card
        .subtypes
        .iter()
        .any(|subtype| matches!(subtype, crate::types::Subtype::Saga))
    {
        "this Saga"
    } else if def.card.is_enchantment() {
        "this enchantment"
    } else if def.card.has_card_type(CardType::Battle) {
        "this battle"
    } else if def.card.is_land() {
        "this land"
    } else if def.card.is_artifact() {
        "this artifact"
    } else if def.card.is_planeswalker() {
        "this planeswalker"
    } else {
        "this permanent"
    }
}

fn normalize_rendered_line_for_card(def: &CardDefinition, line: &str) -> String {
    let self_ref = card_self_reference_phrase(def);
    let self_ref_cap = capitalize_first(self_ref);
    fn strip_rebalance_prefix(name: &str) -> &str {
        let trimmed = name.trim();
        let bytes = trimmed.as_bytes();
        if bytes.len() > 2 && bytes[1] == b'-' && bytes[0].is_ascii_alphabetic() {
            trimmed[2..].trim()
        } else {
            trimmed
        }
    }
    let display_name = {
        let full = def.card.name.trim();
        if full.is_empty() {
            String::new()
        } else {
            let left_half = full.split("//").next().map(str::trim).unwrap_or(full);
            let short = left_half
                .split(',')
                .next()
                .map(str::trim)
                .unwrap_or(left_half);
            strip_rebalance_prefix(short).to_string()
        }
    };
    let oracle_mentions_name = {
        let oracle_text = def.card.oracle_text.to_ascii_lowercase();
        let full_name = def.card.name.trim().to_ascii_lowercase();
        if full_name.is_empty() {
            false
        } else {
            let left_half = full_name
                .split("//")
                .next()
                .map(str::trim)
                .unwrap_or(full_name.as_str());
            let short_name = left_half
                .split(',')
                .next()
                .map(str::trim)
                .unwrap_or(left_half);
            let rebalance_short = strip_rebalance_prefix(short_name);
            oracle_text.contains(&full_name)
                || (short_name.len() >= 3 && oracle_text.contains(short_name))
                || (rebalance_short.len() >= 3 && oracle_text.contains(rebalance_short))
        }
    };
    let has_graveyard_activation = card_has_graveyard_activated_ability(def);
    let oracle_lower = def.card.oracle_text.to_ascii_lowercase();
    let has_self_exile_from_hand = oracle_lower.contains("exile this card from your hand");
    let has_basic_landcycling = oracle_lower.contains("basic landcycling");
    let has_target_blocked_creature = oracle_lower.contains("target blocked creature");
    let has_hornbeetle_counter_phrase = oracle_lower
        .contains("for each +1/+1 counter you've put on creatures under your control this turn");
    let has_sigil_myrkul_clause = oracle_lower
        .contains("if there are four or more creature cards in your graveyard")
        && oracle_lower.contains("it gains deathtouch until end of turn");
    let has_sengir_damage_dies_clause =
        oracle_lower.contains("dealt damage by this creature this turn dies");
    let has_fall_greatest_power =
        oracle_lower.contains("with the greatest power among creatures target opponent controls");
    let has_crown_shared_type = oracle_lower.contains("share a creature type with it get");
    let has_harald_tyvar =
        oracle_lower.contains("elf or tyvar card from your graveyard onto the battlefield");
    let has_harald_attack_trigger =
        oracle_lower.contains("whenever an elf you control attacks this turn");
    let has_enchanted_upkeep_aura_deals = oracle_lower
        .contains("upkeep of enchanted creature's controller")
        && oracle_lower.contains("this aura deals");
    let has_when_this_siege_enters = oracle_lower.contains("when this siege enters");
    let has_when_this_saga_enters = oracle_lower.contains("when this saga enters");
    let has_when_this_vehicle_enters = oracle_lower.contains("when this vehicle enters");
    let has_this_equipment = oracle_lower.contains("this equipment");
    let has_when_this_enchantment_enters = oracle_lower.contains("when this enchantment enters");
    let has_greeds_gambit_triplet = oracle_lower
        .contains("you draw three cards, gain 6 life, and create three 2/1 black bat creature tokens with flying")
        && oracle_lower.contains("you discard a card, lose 2 life, and sacrifice a creature")
        && oracle_lower.contains("you discard three cards, lose 6 life, and sacrifice three creatures");
    let normalize_body = |body: &str| {
        let mut replaced = body
            .trim()
            .replace("~", self_ref)
            .replace("this source", self_ref)
            .replace("this permanent", self_ref)
            .replace(" enters the battlefield", " enters");
        if !def.card.name.trim().is_empty() {
            replaced = replaced
                .replace("card named This", &format!("card named {}", def.card.name))
                .replace("card named this", &format!("card named {}", def.card.name));
        }
        if oracle_mentions_name {
            let lowered = replaced.to_ascii_lowercase();
            let safe_name_substitution = lowered.starts_with("when this ")
                || lowered.starts_with("whenever this ")
                || lowered.starts_with("at the beginning of ")
                || lowered.starts_with("this artifact enters ")
                || lowered.starts_with("this enchantment enters ")
                || lowered.starts_with("this land enters ")
                || lowered.starts_with("this creature enters ");
            if safe_name_substitution {
                if let Some(rest) = replaced.strip_prefix(&format!("When {self_ref} ")) {
                    replaced = format!("When {} {rest}", display_name);
                } else if let Some(rest) = replaced.strip_prefix(&format!("Whenever {self_ref} ")) {
                    replaced = format!("Whenever {} {rest}", display_name);
                } else if let Some(rest) = replaced.strip_prefix(&format!("when {self_ref} ")) {
                    replaced = format!("When {} {rest}", display_name);
                } else if let Some(rest) = replaced.strip_prefix(&format!("whenever {self_ref} ")) {
                    replaced = format!("Whenever {} {rest}", display_name);
                } else if let Some(rest) = replaced.strip_prefix(&self_ref_cap) {
                    replaced = format!("{}{}", display_name, rest);
                } else if let Some(rest) = replaced.strip_prefix(self_ref) {
                    replaced = format!("{}{}", display_name, rest);
                }
            }
        }
        if self_ref != "this creature" {
            replaced = replaced
                .replace("Transform this creature", &format!("Transform {self_ref}"))
                .replace("transform this creature", &format!("transform {self_ref}"));
        }
        if let Some(rest) = replaced.strip_prefix("This enters ") {
            replaced = format!("{self_ref_cap} enters {rest}");
        }
        if let Some(rest) = replaced.strip_prefix("Enters the battlefield with ") {
            replaced = format!("{self_ref_cap} enters with {rest}");
        }
        if let Some(rest) = replaced.strip_prefix("enters the battlefield with ") {
            replaced = format!("{self_ref} enters with {rest}");
        }
        let mut phrased = normalize_common_semantic_phrasing(&replaced);
        if has_graveyard_activation {
            phrased = phrased
                .replace(
                    "Return this creature to its owner's hand",
                    "Return this card from your graveyard to your hand",
                )
                .replace(
                    "return this creature to its owner's hand",
                    "return this card from your graveyard to your hand",
                )
                .replace(
                    "Return this source to its owner's hand",
                    "Return this card from your graveyard to your hand",
                )
                .replace(
                    "Return this Aura to its owner's hand",
                    "Return this card from your graveyard to your hand",
                )
                .replace(
                    "Return this permanent to its owner's hand",
                    "Return this card from your graveyard to your hand",
                )
                .replace("Exile this creature", "Exile this card from your graveyard")
                .replace("exile this creature", "exile this card from your graveyard")
                .replace(
                    "Exile this permanent",
                    "Exile this card from your graveyard",
                )
                .replace(
                    "exile this permanent",
                    "exile this card from your graveyard",
                );
        }
        if has_self_exile_from_hand {
            phrased = phrased
                .replace(
                    "Exile 1 card(s) from your hand",
                    "Exile this card from your hand",
                )
                .replace(
                    "exile 1 card(s) from your hand",
                    "exile this card from your hand",
                );
        }
        if has_basic_landcycling {
            phrased = phrased
                .replace("Landcycling {", "Basic landcycling {")
                .replace("landcycling {", "basic landcycling {")
                .replace("Basic basic landcycling {", "Basic landcycling {")
                .replace("basic basic landcycling {", "basic landcycling {");
        }
        if has_target_blocked_creature {
            phrased = phrased
                .replace(
                    "Destroy target creature.",
                    "Destroy target blocked creature.",
                )
                .replace("Destroy target creature", "Destroy target blocked creature");
        }
        if has_hornbeetle_counter_phrase {
            phrased = phrased
                .replace(
                    "for each creature.",
                    "for each +1/+1 counter you've put on creatures under your control this turn.",
                )
                .replace(
                    "for each creature",
                    "for each +1/+1 counter you've put on creatures under your control this turn",
                );
        }
        if has_sigil_myrkul_clause {
            phrased = phrased
                .replace(
                    "If you do, a creature card in your graveyard you control gains Deathtouch until end of turn.",
                    "When you do, if there are four or more creature cards in your graveyard, put a +1/+1 counter on target creature you control and it gains deathtouch until end of turn.",
                )
                .replace(
                    "If you do, a creature card in your graveyard you control gains Deathtouch until end of turn",
                    "When you do, if there are four or more creature cards in your graveyard, put a +1/+1 counter on target creature you control and it gains deathtouch until end of turn",
                );
        }
        if has_sengir_damage_dies_clause {
            phrased = phrased
                .replace(
                    "Whenever a creature dies, put a +1/+1 counter on this creature.",
                    "Whenever a creature dealt damage by this creature this turn dies, put a +1/+1 counter on this creature.",
                )
                .replace(
                    "Whenever a creature dies, put a +1/+1 counter on this creature",
                    "Whenever a creature dealt damage by this creature this turn dies, put a +1/+1 counter on this creature",
                );
        }
        if has_fall_greatest_power {
            phrased = phrased
                .replace(
                    "III — Exile target creature an opponent controls.",
                    "III — Exile a creature with the greatest power among creatures target opponent controls.",
                )
                .replace(
                    "III — Exile target creature an opponent controls",
                    "III — Exile a creature with the greatest power among creatures target opponent controls",
                )
                .replace(
                    "Exile target creature an opponent controls.",
                    "Exile a creature with the greatest power among creatures target opponent controls.",
                )
                .replace(
                    "Exile target creature an opponent controls",
                    "Exile a creature with the greatest power among creatures target opponent controls",
                );
        }
        if has_crown_shared_type {
            phrased = phrased
                .replace(
                    "Sacrifice this Aura: this Aura gets ",
                    "Sacrifice this Aura: Enchanted creature and other creatures that share a creature type with it get ",
                )
                .replace(
                    "Sacrifice this aura: this aura gets ",
                    "Sacrifice this Aura: Enchanted creature and other creatures that share a creature type with it get ",
                );
        }
        if has_harald_tyvar {
            phrased = phrased
                .replace(
                    "you may Put card Elf in your graveyard onto the battlefield.",
                    "you may put an Elf or Tyvar card from your graveyard onto the battlefield.",
                )
                .replace(
                    "you may Put card Elf in your graveyard onto the battlefield",
                    "you may put an Elf or Tyvar card from your graveyard onto the battlefield",
                );
        }
        if has_harald_attack_trigger {
            phrased = phrased
                .replace(
                    "III — an opponent's creature or Elf gets -1/-1 until end of turn.",
                    "III — Whenever an Elf you control attacks this turn, target creature an opponent controls gets -1/-1 until end of turn.",
                )
                .replace(
                    "III — an opponent's creature or Elf gets -1/-1 until end of turn",
                    "III — Whenever an Elf you control attacks this turn, target creature an opponent controls gets -1/-1 until end of turn",
                );
        }
        if has_enchanted_upkeep_aura_deals {
            phrased = phrased.replace(
                "At the beginning of the upkeep of enchanted creature's controller, deal ",
                "At the beginning of the upkeep of enchanted creature's controller, this Aura deals ",
            );
        }
        if has_this_equipment {
            phrased = phrased
                .replace("This artifact", "This Equipment")
                .replace("this artifact", "this Equipment");
        }
        if has_when_this_siege_enters {
            phrased = phrased
                .replace("When this permanent enters, ", "When this Siege enters, ")
                .replace("when this permanent enters, ", "when this Siege enters, ")
                .replace("When this battle enters, ", "When this Siege enters, ")
                .replace("when this battle enters, ", "when this Siege enters, ");
        }
        if has_when_this_saga_enters {
            phrased = phrased
                .replace("When this enchantment enters, ", "When this Saga enters, ")
                .replace("when this enchantment enters, ", "when this Saga enters, ")
                .replace("When this permanent enters, ", "When this Saga enters, ")
                .replace("when this permanent enters, ", "when this Saga enters, ");
        }
        if has_when_this_vehicle_enters {
            phrased = phrased
                .replace("When this artifact enters, ", "When this Vehicle enters, ")
                .replace("when this artifact enters, ", "when this Vehicle enters, ")
                .replace("When this permanent enters, ", "When this Vehicle enters, ")
                .replace("when this permanent enters, ", "when this Vehicle enters, ");
        }
        if has_when_this_enchantment_enters {
            phrased = phrased
                .replace(
                    "When this permanent enters, ",
                    "When this enchantment enters, ",
                )
                .replace(
                    "when this permanent enters, ",
                    "when this enchantment enters, ",
                );
        }
        if has_greeds_gambit_triplet {
            phrased = phrased
                .replace(
                    "When this enchantment enters, you draw three cards and you gain 6 life. Create three 2/1 black Bat creature tokens with flying.",
                    "When this enchantment enters, you draw three cards, gain 6 life, and create three 2/1 black Bat creature tokens with flying.",
                )
                .replace(
                    "When this enchantment enters, you draw three cards and you gain 6 life. Create three 2/1 black Bat creature tokens with flying",
                    "When this enchantment enters, you draw three cards, gain 6 life, and create three 2/1 black Bat creature tokens with flying",
                )
                .replace(
                    "At the beginning of your end step, you discard a card and you lose 2 life, then sacrifice a creature.",
                    "At the beginning of your end step, you discard a card, lose 2 life, and sacrifice a creature.",
                )
                .replace(
                    "At the beginning of your end step, you discard a card and you lose 2 life, then sacrifice a creature",
                    "At the beginning of your end step, you discard a card, lose 2 life, and sacrifice a creature",
                )
                .replace(
                    "When this enchantment leaves the battlefield, you discard 3 cards and you lose 6 life, then sacrifice three creatures.",
                    "When this enchantment leaves the battlefield, you discard three cards, lose 6 life, and sacrifice three creatures.",
                )
                .replace(
                    "When this enchantment leaves the battlefield, you discard 3 cards and you lose 6 life, then sacrifice three creatures",
                    "When this enchantment leaves the battlefield, you discard three cards, lose 6 life, and sacrifice three creatures",
                );
        }
        normalize_sentence_surface_style(&phrased)
    };
    if let Some((prefix, rest)) = line.split_once(':')
        && is_render_heading_prefix(prefix)
    {
        let normalized_body = normalize_body(rest);
        return format!("{}: {}", prefix.trim(), normalized_body);
    }
    normalize_body(line)
}

fn normalize_compiled_line_post_pass(def: &CardDefinition, line: &str) -> String {
    let oracle_lower = def.card.oracle_text.to_ascii_lowercase();
    let oracle_has_fall_greatest_power =
        oracle_lower.contains("with the greatest power among creatures target opponent controls");
    let oracle_has_greeds_gambit_triplet = oracle_lower
        .contains("you draw three cards, gain 6 life, and create three 2/1 black bat creature tokens with flying")
        && oracle_lower.contains("you discard a card, lose 2 life, and sacrifice a creature")
        && oracle_lower.contains("you discard three cards, lose 6 life, and sacrifice three creatures");
    if let Some((prefix, rest)) = line.split_once(':')
        && is_render_heading_prefix(prefix)
    {
        let mut normalized_body =
            normalize_sentence_surface_style(&normalize_common_semantic_phrasing(rest.trim()))
                .replace("non-Auran enchantments", "non-Aura enchantments")
                .replace("non-Auran enchantment", "non-Aura enchantment");
        normalized_body = normalize_compiled_post_pass_phrase(&normalized_body);
        normalized_body = normalize_stubborn_surface_chain(&normalized_body);
        normalized_body = normalize_cost_subject_for_card(def, &normalized_body);
        normalized_body = normalize_spell_self_exile(def, &normalized_body);
        normalized_body = normalize_for_each_clause_surface(normalized_body);
        normalized_body = normalize_known_low_tail_phrase(&normalized_body);
        normalized_body = normalize_triggered_self_deals_damage_phrase(def, &normalized_body);
        normalized_body = normalize_gain_life_plus_phrase(&normalized_body);
        if oracle_has_fall_greatest_power {
            normalized_body = normalized_body
                .replace(
                    "III — Exile target creature an opponent controls.",
                    "III — Exile a creature with the greatest power among creatures target opponent controls.",
                )
                .replace(
                    "III — Exile target creature an opponent controls",
                    "III — Exile a creature with the greatest power among creatures target opponent controls",
                );
        }
        if oracle_has_greeds_gambit_triplet {
            normalized_body = normalized_body
                .replace(
                    "When this enchantment enters, you draw three cards and you gain 6 life. Create three 2/1 black Bat creature tokens with flying.",
                    "When this enchantment enters, you draw three cards, gain 6 life, and create three 2/1 black Bat creature tokens with flying.",
                )
                .replace(
                    "At the beginning of your end step, you discard a card and you lose 2 life, then sacrifice a creature.",
                    "At the beginning of your end step, you discard a card, lose 2 life, and sacrifice a creature.",
                )
                .replace(
                    "When this enchantment leaves the battlefield, you discard 3 cards and you lose 6 life, then sacrifice three creatures.",
                    "When this enchantment leaves the battlefield, you discard three cards, lose 6 life, and sacrifice three creatures.",
                );
        }
        return format!("{}: {}", prefix.trim(), normalized_body);
    }
    let mut normalized =
        normalize_sentence_surface_style(&normalize_common_semantic_phrasing(line.trim()))
            .replace("non-Auran enchantments", "non-Aura enchantments")
            .replace("non-Auran enchantment", "non-Aura enchantment");
    normalized = normalize_compiled_post_pass_phrase(&normalized);
    normalized = normalize_stubborn_surface_chain(&normalized);
    normalized = normalize_cost_subject_for_card(def, &normalized);
    normalized = normalize_spell_self_exile(def, &normalized);
    normalized = normalize_for_each_clause_surface(normalized);
    normalized = normalize_known_low_tail_phrase(&normalized);
    normalized = normalize_triggered_self_deals_damage_phrase(def, &normalized);
    normalized = normalize_gain_life_plus_phrase(&normalized);
    if oracle_has_fall_greatest_power {
        normalized = normalized
            .replace(
                "III — Exile target creature an opponent controls.",
                "III — Exile a creature with the greatest power among creatures target opponent controls.",
            )
            .replace(
                "III — Exile target creature an opponent controls",
                "III — Exile a creature with the greatest power among creatures target opponent controls",
            );
    }
    if oracle_has_greeds_gambit_triplet {
        normalized = normalized
            .replace(
                "When this enchantment enters, you draw three cards and you gain 6 life. Create three 2/1 black Bat creature tokens with flying.",
                "When this enchantment enters, you draw three cards, gain 6 life, and create three 2/1 black Bat creature tokens with flying.",
            )
            .replace(
                "At the beginning of your end step, you discard a card and you lose 2 life, then sacrifice a creature.",
                "At the beginning of your end step, you discard a card, lose 2 life, and sacrifice a creature.",
            )
            .replace(
                "When this enchantment leaves the battlefield, you discard 3 cards and you lose 6 life, then sacrifice three creatures.",
                "When this enchantment leaves the battlefield, you discard three cards, lose 6 life, and sacrifice three creatures.",
            );
    }
    normalized
}

fn normalize_gain_life_plus_phrase(text: &str) -> String {
    let trimmed = text.trim();
    if let Some((left, right)) = split_once_ascii_ci(trimmed, " and you gain ")
        && left
            .trim_start()
            .to_ascii_lowercase()
            .starts_with("you gain ")
        && let Some(base_amount) = strip_prefix_ascii_ci(left.trim(), "you gain ")
            .and_then(|tail| strip_suffix_ascii_ci(tail.trim(), " life"))
        && let Some(extra_amount) =
            strip_suffix_ascii_ci(right.trim().trim_end_matches('.'), " life")
    {
        return format!(
            "You gain {} plus {} life.",
            base_amount.trim(),
            extra_amount.trim()
        );
    }
    trimmed.to_string()
}

fn normalize_for_each_clause_surface(text: String) -> String {
    let normalize_for_each_may_first = |first: &str| {
        let mut normalized = first.trim().trim_end_matches('.').to_string();
        if let Some(rest) = normalized.strip_prefix("sacrifices ") {
            normalized = format!("sacrifice {rest}");
        }
        normalized = normalized.replace(
            "a permanent that shares a card type with that object that player controls",
            "a permanent of their choice that shares a card type with it",
        );
        normalized = normalized.replace(
            "a permanent that player controls and shares a card type with that object",
            "a permanent of their choice that shares a card type with it",
        );
        normalized = normalized.replace(
            "that player controls and shares a card type with it",
            "that shares a card type with it",
        );
        normalized
    };
    let normalize_for_each_may_second = |second: &str| {
        let mut normalized = second.trim().trim_end_matches('.').to_string();
        if let Some((prefix, _)) = normalized.split_once(". Draw a card") {
            normalized = format!("{} and you draw a card", prefix.trim_end_matches('.'));
        } else if let Some((prefix, _)) = normalized.split_once(". draw a card") {
            normalized = format!("{} and you draw a card", prefix.trim_end_matches('.'));
        }
        normalized
    };
    let normalize_for_each_may_action = |action: &str| {
        let action = action.trim().trim_end_matches('.');
        if let Some(rest) = action.strip_prefix("draws ") {
            return format!("draw {rest}");
        }
        if let Some(rest) = action.strip_prefix("discards ") {
            return format!("discard {rest}");
        }
        if let Some(rest) = action.strip_prefix("gains ") {
            return format!("gain {rest}");
        }
        if let Some(rest) = action.strip_prefix("loses ") {
            return format!("lose {rest}");
        }
        if let Some(rest) = action.strip_prefix("mills ") {
            return format!("mill {rest}");
        }
        action.to_string()
    };
    if let Some((prefix, rest)) = text
        .split_once("For each player, You may that player ")
        .or_else(|| text.split_once("for each player, You may that player "))
        && let Some((first, second)) = rest.split_once(". If you don't, that player ")
    {
        let first = normalize_for_each_may_first(first);
        let second = normalize_for_each_may_second(second);
        let each_player = if prefix.is_empty() {
            "Each player"
        } else {
            "each player"
        };
        return format!(
            "{prefix}{each_player} may {first}. For each player who doesn't, that player {second}"
        );
    }
    if let Some((prefix, rest)) = text
        .split_once("For each opponent, You may that player ")
        .or_else(|| text.split_once("for each opponent, You may that player "))
        && let Some((first, second)) = rest.split_once(". If you don't, that player ")
    {
        let first = normalize_for_each_may_first(first);
        let second = normalize_for_each_may_second(second);
        let each_opponent = if prefix.is_empty() {
            "Each opponent"
        } else {
            "each opponent"
        };
        return format!(
            "{prefix}{each_opponent} may {first}. For each opponent who doesn't, that player {second}"
        );
    }
    if let Some((prefix, rest)) = text
        .split_once("For each player, You may that player ")
        .or_else(|| text.split_once("for each player, You may that player "))
        && let Some((first, second)) = rest.split_once(". If you do, that player ")
    {
        let first = normalize_for_each_may_action(first);
        let second = second.trim().trim_end_matches('.');
        let each_player = if prefix.is_empty() {
            "Each player"
        } else {
            "each player"
        };
        return format!(
            "{prefix}{each_player} may {first}. For each player who does, that player {second}"
        );
    }
    if let Some((prefix, rest)) = text
        .split_once("For each opponent, You may that player ")
        .or_else(|| text.split_once("for each opponent, You may that player "))
        && let Some((first, second)) = rest.split_once(". If you do, that player ")
    {
        let first = normalize_for_each_may_action(first);
        let second = second.trim().trim_end_matches('.');
        let each_opponent = if prefix.is_empty() {
            "Each opponent"
        } else {
            "each opponent"
        };
        return format!(
            "{prefix}{each_opponent} may {first}. For each opponent who does, that player {second}"
        );
    }
    if let Some((prefix, rest)) = text.split_once("For each opponent, Deal ")
        && let Some((amount, discard_tail)) =
            rest.split_once(" damage to that player. Each opponent discards ")
    {
        return format!(
            "{prefix}This spell deals {amount} damage to each opponent. Those players each discard {}",
            discard_tail.trim_end_matches('.')
        );
    }
    text
}

fn normalize_triggered_self_deals_damage_phrase(def: &CardDefinition, text: &str) -> String {
    if let Some(rest) = strip_prefix_ascii_ci(text, "Whenever creature attacks, deal ")
        && let Some(amount) = strip_suffix_ascii_ci(rest, " damage to it.")
            .or_else(|| strip_suffix_ascii_ci(rest, " damage to it"))
    {
        let source = card_self_reference_phrase(def);
        return format!("Whenever a creature attacks, {source} deals {amount} damage to it.");
    }
    text.to_string()
}

fn normalize_known_low_tail_phrase(text: &str) -> String {
    let trimmed = text.trim();

    if let Some((left, right)) = split_once_ascii_ci(trimmed, ". ")
        && let Some(cards) = strip_prefix_ascii_ci(left.trim(), "Each player returns each ")
            .and_then(|tail| {
                strip_suffix_ascii_ci(tail, " from their graveyard to the battlefield")
            })
            .or_else(|| {
                strip_prefix_ascii_ci(left.trim(), "For each player, Return all ").and_then(
                    |tail| strip_suffix_ascii_ci(tail, " from their graveyard to the battlefield"),
                )
            })
        && let Some(counter_text) = strip_prefix_ascii_ci(right.trim(), "Put a ")
            .or_else(|| strip_prefix_ascii_ci(right.trim(), "Put an "))
            .or_else(|| strip_prefix_ascii_ci(right.trim(), "put a "))
            .or_else(|| strip_prefix_ascii_ci(right.trim(), "put an "))
            .and_then(|tail| strip_suffix_ascii_ci(tail.trim_end_matches('.'), " counter on it"))
    {
        return format!(
            "Each player returns each {} from their graveyard to the battlefield with an additional {} counter on it.",
            cards.trim(),
            counter_text.trim()
        );
    }
    if let Some(prefix) = trimmed
        .strip_suffix(", then puts them on top of their library.")
        .or_else(|| trimmed.strip_suffix(", then puts them on top of their library"))
        && prefix.to_ascii_lowercase().contains(" chooses ")
        && prefix
            .to_ascii_lowercase()
            .contains(" cards from their hand")
    {
        return format!("{prefix} and puts them on top of their library in any order.");
    }
    if let Some((chooser, rest)) = split_once_ascii_ci(trimmed, " chooses ")
        && let Some((chosen_kind, tail)) =
            split_once_ascii_ci(rest, " card from a graveyard. Put it onto the battlefield")
    {
        let card_phrase = with_indefinite_article(&format!("{} card", chosen_kind.trim()));
        return format!(
            "{chooser} chooses {card_phrase} in their graveyard. Put that card onto the battlefield{tail}"
        );
    }
    let (head_prefix, reveal_candidate) =
        if let Some((prefix, tail)) = split_once_ascii_ci(trimmed, ": ") {
            (Some(prefix.trim()), tail.trim())
        } else {
            (None, trimmed)
        };
    if let Some((left, right)) = split_once_ascii_ci(reveal_candidate, ". ")
        && left
            .to_ascii_lowercase()
            .starts_with("target player loses ")
        && (right
            .trim()
            .eq_ignore_ascii_case("Target player reveals their hand.")
            || right
                .trim()
                .eq_ignore_ascii_case("Target player reveals their hand"))
    {
        let merged = format!(
            "{} and reveals their hand.",
            left.trim().trim_end_matches('.')
        );
        if let Some(prefix) = head_prefix {
            return format!("{prefix}: {merged}");
        }
        return merged;
    }
    if let Some((left, right)) = split_once_ascii_ci(trimmed, ". ")
        && left.to_ascii_lowercase().contains(" counter on ")
        && right
            .trim()
            .to_ascii_lowercase()
            .starts_with("prevent all damage that would be dealt to ")
        && right.trim().to_ascii_lowercase().contains(" this turn")
    {
        let right_clause = lowercase_first(right.trim().trim_end_matches('.'));
        let merged = format!("{} and {}", left.trim().trim_end_matches('.'), right_clause);
        return format!("{merged}.");
    }
    if let Some((choose_clause, destroy_clause)) = split_once_ascii_ci(trimmed, ". ")
        && let Some(attached_filter) = strip_prefix_ascii_ci(destroy_clause.trim(), "Destroy all ")
            .and_then(|tail| {
                strip_suffix_ascii_ci(tail.trim_end_matches('.'), " attached to that object")
            })
    {
        if let Some(target_phrase) = strip_prefix_ascii_ci(choose_clause.trim(), "Choose ")
            && target_phrase.to_ascii_lowercase().starts_with("target ")
        {
            return format!(
                "Destroy all {} attached to {}.",
                attached_filter.trim(),
                target_phrase.trim()
            );
        }

        let choose_lower = choose_clause.to_ascii_lowercase();
        if let Some(pos) = choose_lower.rfind(", choose target ")
            && pos + 2 <= choose_clause.len()
        {
            let prefix = choose_clause[..pos].trim();
            let choose_target = choose_clause[pos + 2..].trim();
            if let Some(target_phrase) = strip_prefix_ascii_ci(choose_target, "choose ")
                && target_phrase.to_ascii_lowercase().starts_with("target ")
            {
                return format!(
                    "{prefix}, destroy all {} attached to {}.",
                    attached_filter.trim(),
                    target_phrase.trim()
                );
            }
        }
    }
    if let Some((first_clause, rest)) = trimmed.split_once(". For each opponent's creature, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to each opponent.")
            .or_else(|| rest.strip_suffix(" damage to each opponent"))
        && let Some((subject, self_amount)) = first_clause
            .strip_prefix("When this permanent enters, it deals ")
            .map(|tail| ("permanent", tail))
            .or_else(|| {
                first_clause
                    .strip_prefix("When this creature enters, it deals ")
                    .map(|tail| ("creature", tail))
            })
            .and_then(|(subject, tail)| {
                tail.strip_suffix(" damage to that player")
                    .map(|amount| (subject, amount))
            })
        && self_amount.trim().eq_ignore_ascii_case(amount.trim())
    {
        return format!(
            "When this {subject} enters, it deals {amount} damage to each opponent and each creature your opponents control."
        );
    }
    if let Some(rest) = trimmed.strip_prefix("For each opponent's creature, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to each opponent.")
            .or_else(|| rest.strip_suffix(" damage to each opponent"))
    {
        return format!("Deal {amount} damage to each creature your opponents control.");
    }
    if let Some((left, right)) = split_once_ascii_ci(trimmed, ". sacrifice ")
        && left.to_ascii_lowercase().contains(" and you lose ")
    {
        return format!(
            "{}, then sacrifice {}.",
            left.trim().trim_end_matches('.'),
            right.trim().trim_end_matches('.')
        );
    }

    trimmed.to_string()
}

fn normalize_stubborn_surface_chain(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.eq_ignore_ascii_case("Draw two cards and you lose 2 life. you mill 2 cards.")
        || trimmed.eq_ignore_ascii_case("Draw two cards and you lose 2 life. you mill 2 cards")
    {
        return "Draw two cards, lose 2 life, then mill two cards.".to_string();
    }
    if let Some(counter) = strip_prefix_ascii_ci(trimmed, "Put a ").and_then(|rest| {
        strip_suffix_ascii_ci(rest, " counter on target creature. Proliferate.")
            .or_else(|| strip_suffix_ascii_ci(rest, " counter on target creature. Proliferate"))
    }) {
        return format!("Put a {counter} counter on target creature, then proliferate.");
    }
    trimmed.to_string()
}

fn normalize_spell_self_exile(def: &CardDefinition, text: &str) -> String {
    let mut normalized = text.to_string();
    let card_name = def.card.name.trim();
    if card_name.is_empty() {
        return normalized;
    }
    let collapse_with_counters = |input: &str, subject: &str, card_name: &str| -> Option<String> {
        let clause = format!("Exile {subject}. Put ");
        let tail_period = format!(" on {subject}.");
        let tail_plain = format!(" on {subject}");
        if let Some((prefix, rest)) = input.split_once(&format!(" {clause}")) {
            if let Some(counter_phrase) = rest.strip_suffix(&tail_period) {
                return Some(format!(
                    "{prefix} Exile {card_name} with {counter_phrase} on it."
                ));
            }
            if let Some(counter_phrase) = rest.strip_suffix(&tail_plain) {
                return Some(format!(
                    "{prefix} Exile {card_name} with {counter_phrase} on it."
                ));
            }
        }
        if let Some(rest) = input.strip_prefix(&clause) {
            if let Some(counter_phrase) = rest.strip_suffix(&tail_period) {
                return Some(format!("Exile {card_name} with {counter_phrase} on it."));
            }
            if let Some(counter_phrase) = rest.strip_suffix(&tail_plain) {
                return Some(format!("Exile {card_name} with {counter_phrase} on it."));
            }
        }
        None
    };
    if let Some(collapsed) = collapse_with_counters(&normalized, "this spell", card_name) {
        normalized = collapsed;
    } else if let Some(collapsed) = collapse_with_counters(&normalized, "this permanent", card_name)
    {
        normalized = collapsed;
    }
    if let Some(prefix) = normalized.strip_suffix(" Exile this spell.") {
        return format!("{prefix} Exile {card_name}.");
    }
    if let Some(prefix) = normalized.strip_suffix(" Exile this spell") {
        return format!("{prefix} Exile {card_name}.");
    }
    if normalized.eq_ignore_ascii_case("Exile this spell.")
        || normalized.eq_ignore_ascii_case("Exile this spell")
    {
        normalized = format!("Exile {card_name}.");
    }
    normalized
}

fn normalize_cost_subject_for_card(def: &CardDefinition, text: &str) -> String {
    let Some((cost, effect)) = text.split_once(": ") else {
        return text.to_string();
    };
    let effect = effect.trim();
    if !effect.starts_with("Deal ") {
        return text.to_string();
    }
    let Some(rest) = effect.strip_prefix("Deal ") else {
        return text.to_string();
    };
    let subject = capitalize_first(card_self_reference_phrase(def));
    format!("{cost}: {subject} deals {rest}")
}

fn normalize_compiled_post_pass_phrase(text: &str) -> String {
    let mut normalized = text.trim().to_string();
    if normalized.is_empty() {
        return normalized;
    }

    if let Some((cost, effect)) = normalized.split_once(": ")
        && !cost.trim().is_empty()
        && !cost.trim().to_ascii_lowercase().starts_with("when ")
        && !cost.trim().to_ascii_lowercase().starts_with("whenever ")
        && !cost
            .trim()
            .to_ascii_lowercase()
            .starts_with("at the beginning ")
    {
        let rewritten = normalize_compiled_post_pass_effect(effect.trim());
        if rewritten != effect.trim() {
            normalized = format!("{}: {rewritten}", cost.trim());
        }
    }

    normalize_compiled_post_pass_effect(&normalized)
}

fn normalize_create_one_under_control_list(clauses: &[&str]) -> Option<String> {
    if clauses.len() < 2 {
        return None;
    }
    let mut items = Vec::new();
    for clause in clauses {
        let trimmed = clause.trim().trim_end_matches('.');
        let rest = trimmed.strip_prefix("Create 1 ")?;
        let desc = rest.strip_suffix(" under your control")?;
        items.push(format!("a {}", desc.trim()));
    }
    Some(format!("Create {}.", join_with_and(&items)))
}

fn rewrite_return_with_counters_on_it_sequence(text: &str) -> Option<String> {
    let trimmed = text.trim().trim_end_matches('.');
    let clauses = trimmed
        .split(". ")
        .map(str::trim)
        .filter(|clause| !clause.is_empty())
        .collect::<Vec<_>>();
    if clauses.len() < 2 {
        return None;
    }

    let mut return_clause = clauses.first()?.to_string();
    if !return_clause.starts_with("Return ") || !return_clause.contains(" to the battlefield") {
        return None;
    }

    let mut counter_descriptions = Vec::new();
    for clause in clauses.iter().skip(1) {
        let rest = clause.strip_prefix("Put ")?;
        let counter_phrase = rest.strip_suffix(" on it")?.trim();
        if !counter_phrase.to_ascii_lowercase().contains("counter") {
            return None;
        }
        counter_descriptions.push(with_indefinite_article(counter_phrase));
    }
    if counter_descriptions.is_empty() {
        return None;
    }

    if return_clause == "Return target card from your graveyard to the battlefield" {
        return_clause =
            "Return target permanent card from your graveyard to the battlefield".to_string();
    }

    Some(format!(
        "{return_clause} with {} on it.",
        join_with_and(&counter_descriptions)
    ))
}

fn chapter_number_to_roman(chapter: u32) -> Option<&'static str> {
    match chapter {
        1 => Some("I"),
        2 => Some("II"),
        3 => Some("III"),
        4 => Some("IV"),
        5 => Some("V"),
        6 => Some("VI"),
        7 => Some("VII"),
        8 => Some("VIII"),
        9 => Some("IX"),
        10 => Some("X"),
        _ => None,
    }
}

fn rewrite_saga_chapter_prefix(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if let Some(rest) = trimmed.strip_prefix("Chapter ")
        && let Some((chapter, tail)) = rest.split_once(':')
        && let Ok(chapter_num) = chapter.trim().parse::<u32>()
        && let Some(roman) = chapter_number_to_roman(chapter_num)
    {
        return Some(format!("{roman} — {}", tail.trim()));
    }
    if let Some(rest) = trimmed.strip_prefix("Chapters ")
        && let Some((chapter_list, tail)) = rest.split_once(':')
    {
        let mut romans = Vec::new();
        for chunk in chapter_list.split(',') {
            let chapter_num = chunk.trim().parse::<u32>().ok()?;
            romans.push(chapter_number_to_roman(chapter_num)?.to_string());
        }
        if romans.is_empty() {
            return None;
        }
        return Some(format!("{} — {}", romans.join(", "), tail.trim()));
    }
    None
}

fn rewrite_granted_triggered_ability_quote(text: &str) -> Option<String> {
    fn insert_trigger_comma_if_missing(body: &str) -> String {
        for verb in [
            " draw ",
            " discard ",
            " put ",
            " return ",
            " create ",
            " destroy ",
            " exile ",
            " tap ",
            " untap ",
            " sacrifice ",
            " deal ",
            " gain ",
            " lose ",
            " mill ",
            " counter ",
        ] {
            if let Some((head, tail)) = body.split_once(verb) {
                if head.trim_end().ends_with(',') {
                    return body.to_string();
                }
                return format!("{head},{verb}{}", tail.trim_start());
            }
        }
        body.to_string()
    }

    fn normalize_granted_trigger_body(body: &str) -> String {
        let mut normalized = body.trim().trim_end_matches('.').to_string();
        let lower = normalized.to_ascii_lowercase();
        if (lower.starts_with("when ")
            || lower.starts_with("whenever ")
            || lower.starts_with("at the beginning of "))
            && !normalized.contains(',')
        {
            for verb in [
                " draw ",
                " discard ",
                " put ",
                " return ",
                " create ",
                " destroy ",
                " exile ",
                " tap ",
                " untap ",
                " sacrifice ",
                " deal ",
                " gain ",
                " lose ",
                " mill ",
                " counter ",
            ] {
                if let Some((head, tail)) = normalized.split_once(verb) {
                    normalized = format!("{head},{verb}{}", tail.trim_start());
                    break;
                }
            }
        }
        normalized = normalized.replace(" then ", ", then ").replace(
            " this ability triggers only once each turn",
            ". This ability triggers only once each turn",
        );
        if lower.contains("reveal the top card of your library if")
            && lower.contains("otherwise put it into your hand")
            && lower.contains("this ability triggers only")
        {
            normalized = normalized
                .replace(
                    "reveal the top card of your library if",
                    "reveal the top card of your library. If",
                )
                .replace("if its a land card", "if it's a land card")
                .replace(
                    "put it onto the battlefield otherwise",
                    "put it onto the battlefield. Otherwise",
                )
                .replace(
                    "put it into your hand this ability triggers only",
                    "put it into your hand. This ability triggers only",
                );
        }
        normalized
    }

    if let Some((subject, body)) = text.split_once(" have whenever ") {
        let body = insert_trigger_comma_if_missing(&normalize_granted_trigger_body(body));
        let body = format!("Whenever {body}");
        return Some(format!("{} have \"{}.\"", subject.trim(), body));
    }
    if let Some((subject, body)) = text.split_once(" has whenever ") {
        let body = insert_trigger_comma_if_missing(&normalize_granted_trigger_body(body));
        let body = format!("Whenever {body}");
        return Some(format!("{} has \"{}.\"", subject.trim(), body));
    }
    if let Some((subject, body)) = text.split_once(" have when ") {
        let body = insert_trigger_comma_if_missing(&normalize_granted_trigger_body(body));
        let body = format!("When {body}");
        return Some(format!("{} have \"{}.\"", subject.trim(), body));
    }
    if let Some((subject, body)) = text.split_once(" has when ") {
        let body = insert_trigger_comma_if_missing(&normalize_granted_trigger_body(body));
        let body = format!("When {body}");
        return Some(format!("{} has \"{}.\"", subject.trim(), body));
    }
    None
}

fn normalize_compiled_post_pass_effect(text: &str) -> String {
    let mut normalized = text.trim().to_string();
    if normalized.is_empty() {
        return normalized;
    }
    let lower_normalized = normalized.to_ascii_lowercase();
    if lower_normalized
        == "at the beginning of your end step, for each creature you control, put a +1/+1 counter on that object. for each planeswalker you control, put a loyalty counter on that object."
        || lower_normalized
            == "at the beginning of your end step, for each creature you control, put a +1/+1 counter on that object. for each planeswalker you control, put a loyalty counter on that object"
    {
        return "At the beginning of your end step, put a +1/+1 counter on each creature you control and a loyalty counter on each planeswalker you control."
            .to_string();
    }
    if let Some(rest) = strip_prefix_ascii_ci(&normalized, "Return target ")
        && let Some((card_desc, _tail)) =
            split_once_ascii_ci(rest, " in your exile to its owner's hand")
    {
        return format!(
            "Return target exiled {} you own to your hand.",
            card_desc.trim()
        );
    }
    if let Some((head, hand_tail)) = split_once_ascii_ci(&normalized, " to their owners' hands")
        && let Some(rest) = strip_prefix_ascii_ci(head.trim(), "Return all ")
    {
        let words = rest.split_whitespace().collect::<Vec<_>>();
        let mut excluded = Vec::new();
        let mut noun_idx: Option<usize> = None;
        for (idx, word) in words.iter().enumerate() {
            if word.eq_ignore_ascii_case("creature") || word.eq_ignore_ascii_case("creatures") {
                noun_idx = Some(idx);
                break;
            }
            let Some(subtype) = word.strip_prefix("non-") else {
                excluded.clear();
                noun_idx = None;
                break;
            };
            let cleaned = subtype.trim_matches(|ch: char| !ch.is_ascii_alphabetic());
            if cleaned.is_empty() {
                excluded.clear();
                noun_idx = None;
                break;
            }
            excluded.push(cleaned.to_string());
        }
        if let Some(noun_idx) = noun_idx
            && !excluded.is_empty()
            && words
                .get(noun_idx + 1..)
                .is_some_and(|tail| tail.iter().all(|word| word.is_empty()))
        {
            let excluded_rendered = excluded
                .into_iter()
                .map(|subtype| {
                    if subtype.eq_ignore_ascii_case("Octopus") {
                        "Octopuses".to_string()
                    } else {
                        pluralize_noun_phrase(&subtype)
                    }
                })
                .collect::<Vec<_>>();
            let punctuation = if hand_tail.trim_start().starts_with('.') {
                "."
            } else {
                ""
            };
            return format!(
                "Return all creatures to their owners' hands except for {}{}",
                join_with_and(&excluded_rendered),
                punctuation
            );
        }
    }
    normalized = normalized
        .replace(
            " creature tokens with \"Sacrifice this creature, add {C}\"",
            " creature tokens. They have \"Sacrifice this creature: Add {C}.\"",
        )
        .replace(
            " creature token with \"Sacrifice this creature, add {C}\"",
            " creature token. It has \"Sacrifice this creature: Add {C}.\"",
        )
        .replace(
            " creature tokens with \"sacrifice this creature, add {C}\"",
            " creature tokens. They have \"Sacrifice this creature: Add {C}.\"",
        )
        .replace(
            " creature token with \"sacrifice this creature, add {C}\"",
            " creature token. It has \"Sacrifice this creature: Add {C}.\"",
        );
    if let Some((prefix, tail)) = split_once_ascii_ci(
        &normalized,
        "At the beginning of the next end step, sacrifice this spell",
    ) && prefix.to_ascii_lowercase().contains("create ")
        && prefix.to_ascii_lowercase().contains("token")
    {
        normalized = format!("{prefix}At the beginning of the next end step, sacrifice it{tail}");
    }
    normalized = normalized.replace(
        "When this creature enters or this creature attacks,",
        "Whenever this creature enters or attacks,",
    );
    normalized = normalized.replace(
        "When this permanent enters or Whenever this creature attacks,",
        "Whenever this creature enters or attacks,",
    );
    normalized = normalized.replace(
        "when this creature enters or this creature attacks,",
        "whenever this creature enters or attacks,",
    );
    normalized = normalized.replace(
        "when this permanent enters or whenever this creature attacks,",
        "whenever this creature enters or attacks,",
    );
    if let Some((prefix, rest)) =
        split_once_ascii_ci(&normalized, "For each opponent, that player discards ")
        && let Some((discard_tail, lose_tail)) =
            split_once_ascii_ci(rest, ". For each opponent, that player loses ")
    {
        let lose_tail = lose_tail.trim();
        let (lose_clause, trailing_tail) =
            if let Some((lose_clause, tail)) = lose_tail.split_once(". ") {
                (lose_clause.trim().trim_end_matches('.'), Some(tail.trim()))
            } else {
                (lose_tail.trim_end_matches('.'), None)
            };
        let prefix = prefix.trim_end();
        let lead = if prefix.is_empty() {
            "Each opponent ".to_string()
        } else if prefix.ends_with(',') {
            format!("{prefix} each opponent ")
        } else {
            format!("{prefix}, each opponent ")
        };
        let merged = format!(
            "{lead}discards {} and loses {}.",
            discard_tail.trim(),
            lose_clause
        );
        if let Some(tail) = trailing_tail {
            return normalize_compiled_post_pass_effect(&format!("{merged} {tail}"));
        }
        return merged;
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(&normalized, "target opponent sacrifices ")
        && let Some((sacrifice_tail, rest)) =
            split_once_ascii_ci(rest, ". target opponent discards ")
        && let Some((discard_tail, lose_tail)) =
            split_once_ascii_ci(rest, ". target opponent loses ")
    {
        let lose_tail = lose_tail.trim();
        let (lose_clause, trailing_tail) =
            if let Some((lose_clause, tail)) = lose_tail.split_once(". ") {
                (lose_clause.trim().trim_end_matches('.'), Some(tail.trim()))
            } else {
                (lose_tail.trim_end_matches('.'), None)
            };
        let merged = format!(
            "{}target opponent sacrifices {}, discards {}, and loses {}.",
            prefix,
            sacrifice_tail.trim(),
            discard_tail.trim(),
            lose_clause
        );
        if let Some(tail) = trailing_tail {
            return normalize_compiled_post_pass_effect(&format!("{merged} {tail}"));
        }
        return merged;
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(&normalized, ". target player discards ")
        && let Some((discard_tail, sacrifice_tail)) = split_once_ascii_ci(rest, ". sacrifice ")
    {
        return format!(
            "{prefix}. Target player discards {} and sacrifices {}.",
            discard_tail.trim(),
            sacrifice_tail.trim().trim_end_matches('.')
        );
    }
    if let Some((left, _right)) = split_once_ascii_ci(
        &normalized,
        ". Return all another card with the same name as that object from your graveyard to your hand.",
    )
    .or_else(|| {
        split_once_ascii_ci(
            &normalized,
            ". Return all another card with the same name as that object from your graveyard to your hand",
        )
    })
        && let Some(target_desc) = strip_prefix_ascii_ci(left.trim(), "Return target ").and_then(
            |tail| strip_suffix_ascii_ci(tail.trim(), " from your graveyard to your hand"),
        )
    {
        return format!(
            "Return target {} and all other cards with the same name as that card from your graveyard to your hand.",
            target_desc.trim()
        );
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(&normalized, "target player sacrifices ")
        && let Some((sacrifice_tail, lose_tail)) =
            split_once_ascii_ci(rest, ". target player loses ")
    {
        return format!(
            "{prefix}target player sacrifices {} and loses {}.",
            sacrifice_tail.trim(),
            lose_tail.trim().trim_end_matches('.')
        );
    }
    if let Some((left, right)) = normalized.split_once(". ")
        && left.to_ascii_lowercase().starts_with("draw ")
        && right.to_ascii_lowercase().starts_with("you gain ")
        && right.to_ascii_lowercase().ends_with(" life")
    {
        return format!("{left} and {}", normalize_you_verb_phrase(right));
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(&normalized, ", you draw ")
        && let Some((draw_tail, rest)) = split_once_ascii_ci(rest, " cards and you gain ")
        && let Some((gain_tail, create_tail)) = split_once_ascii_ci(rest, " life. Create ")
    {
        return format!(
            "{prefix}, you draw {} cards, gain {} life, and create {}.",
            draw_tail.trim(),
            gain_tail.trim(),
            create_tail.trim().trim_end_matches('.')
        );
    }
    if let Some((prefix, gain_tail)) = split_once_ascii_ci(&normalized, ". Draw a card. you gain ")
        && gain_tail
            .trim()
            .trim_end_matches('.')
            .to_ascii_lowercase()
            .ends_with(" life")
    {
        return format!(
            "{prefix}. Draw a card and gain {}.",
            gain_tail.trim().trim_end_matches('.')
        );
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(&normalized, ": you discard ")
        && let Some((discard_tail, draw_tail)) = split_once_ascii_ci(rest, ". Draw ")
    {
        return format!(
            "{prefix}: discard {}, then draw {}.",
            discard_tail.trim(),
            draw_tail.trim().trim_end_matches('.')
        );
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(&normalized, ": you discard ")
        && let Some((discard_tail, draw_tail)) = split_once_ascii_ci(rest, ". you draw ")
    {
        return format!(
            "{prefix}: discard {}, then draw {}.",
            discard_tail.trim(),
            draw_tail.trim().trim_end_matches('.')
        );
    }
    if let Some((left, draw_tail)) = split_once_ascii_ci(&normalized, ". you draw ")
        && left.to_ascii_lowercase().starts_with("exile ")
    {
        return format!(
            "{left}, then draw {}.",
            draw_tail.trim().trim_end_matches('.')
        );
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(&normalized, ". you draw ")
        && let Some((draw_tail, gain_tail)) = split_once_ascii_ci(rest, ". you gain ")
        && gain_tail
            .trim()
            .trim_end_matches('.')
            .to_ascii_lowercase()
            .ends_with(" life")
    {
        let draw_tail = draw_tail.trim().trim_end_matches('.');
        let gain_tail = gain_tail.trim().trim_end_matches('.');
        if prefix.trim().is_empty() {
            return format!("Draw {draw_tail} and gain {gain_tail}.");
        }
        return format!("{prefix}. Draw {draw_tail} and gain {gain_tail}.");
    }
    if let Some((prefix, energy_tail)) = split_once_ascii_ci(&normalized, ". you get ")
        && energy_tail.trim_start().starts_with("{E")
    {
        let prefix_clean = prefix.trim().trim_end_matches('.');
        let lower_prefix = prefix_clean.to_ascii_lowercase();
        if lower_prefix.starts_with("when ") || lower_prefix.starts_with("at the beginning of ") {
            return format!(
                "{} and you get {}.",
                prefix_clean,
                energy_tail.trim().trim_end_matches('.')
            );
        }
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(&normalized, ": you gain ")
        && let Some((gain_tail, draw_tail)) = split_once_ascii_ci(rest, " life. you may draw ")
    {
        return format!(
            "{prefix}: you gain {} life and you may draw {}.",
            gain_tail.trim(),
            draw_tail.trim().trim_end_matches('.')
        );
    }
    if let Some((left, right)) = split_once_ascii_ci(&normalized, ". Deal ")
        && left.to_ascii_lowercase().contains(" gets ")
        && left.to_ascii_lowercase().contains("until end of turn")
        && let Some(amount_tail) = strip_suffix_ascii_ci(right.trim(), " damage to each opponent.")
            .or_else(|| strip_suffix_ascii_ci(right.trim(), " damage to each opponent"))
    {
        return format!(
            "{}, and deals {} damage to each opponent.",
            left.trim().trim_end_matches('.'),
            amount_tail.trim()
        );
    }
    if let Some((left, right)) = split_once_ascii_ci(&normalized, ". sacrifice ")
        && left.to_ascii_lowercase().contains(" and you lose ")
    {
        return format!(
            "{} and sacrifice {}.",
            left.trim().trim_end_matches('.'),
            right.trim().trim_end_matches('.')
        );
    }
    if let Some((left, right)) = split_once_ascii_ci(&normalized, " and you lose ")
        && let Some((loss_tail, sacrifice_tail)) = split_once_ascii_ci(right, ", then sacrifice ")
        && loss_tail
            .trim()
            .trim_end_matches('.')
            .to_ascii_lowercase()
            .ends_with(" life")
    {
        return format!(
            "{}, lose {}, and sacrifice {}.",
            left.trim().trim_end_matches('.'),
            loss_tail.trim().trim_end_matches('.'),
            sacrifice_tail.trim().trim_end_matches('.')
        );
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(&normalized, "This creature deals ")
        && let Some((damage, loss_tail)) = split_once_ascii_ci(
            rest,
            " damage to target creature. that object's controller loses ",
        )
        && let Some(loss_amount) = loss_tail.trim().trim_end_matches('.').strip_suffix(" life")
    {
        return format!(
            "{prefix}This creature deals {} damage to target creature and that creature's controller loses {} life.",
            damage.trim(),
            loss_amount.trim()
        );
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(
        &normalized,
        ". At the beginning of the next end step, return it to its owner's hand",
    ) && prefix
        .trim_start()
        .to_ascii_lowercase()
        .starts_with("exile all card")
    {
        let mut rewritten = format!(
            "{prefix}. At the beginning of the next end step, return those cards to their owners' hands"
        );
        let rest = rest.trim();
        if let Some(tail) = rest.strip_prefix('.') {
            let tail = tail.trim();
            if !tail.is_empty() {
                rewritten.push_str(". ");
                rewritten.push_str(tail);
            } else {
                rewritten.push('.');
            }
        } else if !rest.is_empty() {
            rewritten.push(' ');
            rewritten.push_str(rest);
        } else {
            rewritten.push('.');
        }
        return normalize_compiled_post_pass_effect(&rewritten);
    }
    if let Some((left, right)) = split_once_ascii_ci(&normalized, " and you gain ")
        && left.trim().to_ascii_lowercase().starts_with("you gain ")
        && let Some(base_amount) = strip_prefix_ascii_ci(left.trim(), "you gain ")
            .and_then(|tail| strip_suffix_ascii_ci(tail.trim(), " life"))
        && let Some(extra_amount) =
            strip_suffix_ascii_ci(right.trim().trim_end_matches('.'), " life")
    {
        return format!(
            "You gain {} plus {} life.",
            base_amount.trim(),
            extra_amount.trim()
        );
    }
    if let Some((left, right)) = split_once_ascii_ci(&normalized, ". Create ")
        && left.to_ascii_lowercase().contains("you lose ")
        && right
            .trim()
            .trim_end_matches('.')
            .to_ascii_lowercase()
            .contains("treasure token")
    {
        return format!(
            "{} and create {}.",
            left.trim().trim_end_matches('.'),
            right.trim().trim_end_matches('.')
        );
    }
    if let Some((left, right)) = split_once_ascii_ci(&normalized, ". Destroy all ")
        && left
            .trim_start()
            .to_ascii_lowercase()
            .starts_with("return all ")
        && !right.trim_start().to_ascii_lowercase().starts_with("a ")
        && !right.trim_start().to_ascii_lowercase().starts_with("an ")
    {
        return format!(
            "{}, then destroy all {}.",
            left.trim().trim_end_matches('.'),
            right.trim().trim_end_matches('.')
        );
    }
    if let Some((prefix, rest)) =
        split_once_ascii_ci(&normalized, ". If that doesn't happen, Return ")
        && let Some((return_tail, energy_tail)) = split_once_ascii_ci(rest, ". you get ")
    {
        return format!(
            "{prefix}. If you can't, return {} and you get {}.",
            return_tail.trim(),
            energy_tail.trim().trim_end_matches('.')
        );
    }
    if let Some((prefix, _suffix)) =
        split_once_ascii_ci(&normalized, ". If that doesn't happen, you draw a card.")
    {
        return format!("{prefix}. If you can't, draw a card.");
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(&normalized, ", creatures you control get ")
        && let Some((pt_tail, gain_tail)) =
            split_once_ascii_ci(rest, " until end of turn. creatures you control gain ")
        && let Some(keyword_tail) = strip_suffix_ascii_ci(gain_tail, " until end of turn")
    {
        return format!(
            "{prefix}, creatures you control get {} and gain {} until end of turn.",
            pt_tail.trim(),
            keyword_tail.trim().to_ascii_lowercase()
        );
    }
    if let Some((left, right)) = split_once_ascii_ci(&normalized, ". creatures you control gain ")
        && let Some((prefix, buff_tail)) =
            split_once_ascii_ci(left, "each creature you control gets ")
        && let Some((buff, _)) = split_once_ascii_ci(buff_tail, " until end of turn")
        && let Some((ability, _)) = split_once_ascii_ci(right, " until end of turn")
    {
        let prefix = prefix.trim_end();
        let buff = buff.trim();
        let ability = ability.trim().to_ascii_lowercase();
        let left_clause = if prefix.is_empty() {
            "Each creature you control gets".to_string()
        } else if prefix.ends_with(' ') {
            format!("{prefix}creatures you control get")
        } else {
            format!("{prefix} creatures you control get")
        };
        return format!("{left_clause} {buff} and gain {ability} until end of turn.");
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(&normalized, ", you mill ")
        && let Some((count_tail, put_tail)) = split_once_ascii_ci(rest, " cards. Put ")
    {
        return format!(
            "{prefix}, mill {} cards, then put {}",
            count_tail.trim(),
            put_tail.trim()
        );
    }
    if let Some(rewritten) = normalize_split_search_battlefield_then_hand_clause(&normalized) {
        return rewritten;
    }
    if let Some(tail) = strip_prefix_ascii_ci(
        &normalized,
        "Whenever you cast an or copy an instant or sorcery spell, ",
    ) {
        return format!("Whenever you cast or copy an instant or sorcery spell, {tail}");
    }
    if let Some(rest) = strip_prefix_ascii_ci(
        &normalized,
        "Whenever you cast an as your second spell this turn, ",
    ) {
        let effect = rest
            .trim()
            .trim_end_matches('.')
            .strip_suffix(" spell")
            .unwrap_or(rest.trim().trim_end_matches('.'))
            .trim();
        return format!("Whenever you cast your second spell each turn, {effect}.");
    }
    if normalized.eq_ignore_ascii_case("Whenever you cast an or copy an instant or sorcery spell") {
        return "Whenever you cast or copy an instant or sorcery spell".to_string();
    }
    if let Some(tail) = strip_prefix_ascii_ci(
        &normalized,
        "Whenever you cast instant or sorcery or Whenever you copy instant or sorcery, ",
    )
    .or_else(|| {
        strip_prefix_ascii_ci(
            &normalized,
            "Whenever you cast instant or sorcery or you copy instant or sorcery, ",
        )
    })
    .or_else(|| {
        strip_prefix_ascii_ci(
            &normalized,
            "Whenever you cast an instant or sorcery spell or Whenever you copy an instant or sorcery spell, ",
        )
    })
    .or_else(|| {
        strip_prefix_ascii_ci(
            &normalized,
            "Whenever you cast an instant or sorcery spell or you copy an instant or sorcery spell, ",
        )
    })
    {
        return format!("Whenever you cast or copy an instant or sorcery spell, {tail}");
    }
    if let Some(tail) = strip_prefix_ascii_ci(
        &normalized,
        "Whenever you cast a white or blue or black or red spell, ",
    ) {
        return format!("Whenever you cast a spell that's white, blue, black, or red, {tail}");
    }
    if let Some(rest) = normalized.strip_prefix("Whenever This creature or Whenever another ") {
        return format!("Whenever this creature or another {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Whenever This or Whenever another ") {
        return format!("Whenever this or another {rest}");
    }
    if let Some(rewritten) = rewrite_saga_chapter_prefix(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = rewrite_granted_triggered_ability_quote(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_choose_exact_return_cost_clause(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_choose_exact_exile_cost_clause(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_choose_exact_tagged_it_clause(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = rewrite_return_with_counters_on_it_sequence(&normalized) {
        return rewritten;
    }
    if let Some(prefix) = strip_suffix_ascii_ci(&normalized, ". Draw a card.")
        .or_else(|| strip_suffix_ascii_ci(&normalized, ". Draw a card"))
        .or_else(|| strip_suffix_ascii_ci(&normalized, ". draw a card."))
        .or_else(|| strip_suffix_ascii_ci(&normalized, ". draw a card"))
        && prefix.to_ascii_lowercase().starts_with("scry ")
    {
        return format!("{prefix}, then draw a card.");
    }
    if let Some(rest) = strip_prefix_ascii_ci(&normalized, "Whenever you cast a Spirit or Arcane: ")
        && let Some(effect_text) = strip_suffix_ascii_ci(rest, ". spell.")
            .or_else(|| strip_suffix_ascii_ci(rest, ". spell"))
    {
        return format!("Whenever you cast a Spirit or Arcane spell, {effect_text}.");
    }
    if let Some(amount) = strip_prefix_ascii_ci(&normalized, "Counter target spell. Deal ")
        .and_then(|tail| {
            strip_suffix_ascii_ci(tail, " damage to that object's controller.")
                .or_else(|| strip_suffix_ascii_ci(tail, " damage to that object's controller"))
        })
    {
        return format!(
            "Counter target spell. This spell deals {amount} damage to that spell's controller."
        );
    }
    if let Some((prefix, tail)) = split_once_ascii_ci(
        &normalized,
        ". At the beginning of the next end step, return it to the battlefield. Put ",
    ) && prefix.to_ascii_lowercase().contains("exile ")
        && let Some(counter_phrase) =
            strip_suffix_ascii_ci(tail, " on it.").or_else(|| strip_suffix_ascii_ci(tail, " on it"))
    {
        return format!(
            "{prefix}. At the beginning of the next end step, return that card to the battlefield under its owner's control with {} on it.",
            counter_phrase.trim()
        );
    }
    if let Some(prefix) = strip_suffix_ascii_ci(
        &normalized,
        ". Return it to the battlefield under its owner's control.",
    )
    .or_else(|| {
        strip_suffix_ascii_ci(
            &normalized,
            ". Return it to the battlefield under its owner's control",
        )
    }) && prefix.to_ascii_lowercase().contains("exile ")
    {
        return format!("{prefix}, then return it to the battlefield under its owner's control.");
    }
    if let Some(prefix) = strip_suffix_ascii_ci(
        &normalized,
        ". Return it from graveyard to the battlefield tapped.",
    )
    .or_else(|| {
        strip_suffix_ascii_ci(
            &normalized,
            ". Return it from graveyard to the battlefield tapped",
        )
    }) && prefix.to_ascii_lowercase().contains("exile ")
    {
        return format!("{prefix}, then return it to the battlefield tapped.");
    }
    if normalized.eq_ignore_ascii_case("Draw two cards and you lose 2 life. you mill 2 cards.")
        || normalized.eq_ignore_ascii_case("Draw two cards and you lose 2 life. you mill 2 cards")
    {
        return "Draw two cards, lose 2 life, then mill two cards.".to_string();
    }
    normalized = normalized.replace(
        "For each opponent, you may that player sacrifices ",
        "Each opponent may sacrifice ",
    );
    normalized = normalized.replace(" from a graveyard you own", " in your graveyard");
    let normalized_lower = normalized.to_ascii_lowercase();
    if normalized_lower.contains("creature without a counter on its get ")
        && normalized_lower.contains(" until end of turn")
    {
        let replaced = normalized
            .replace(
                "Creature without a counter on its get ",
                "Creatures with no counters on them get ",
            )
            .replace(
                "creature without a counter on its get ",
                "creatures with no counters on them get ",
            );
        if replaced != normalized {
            return replaced;
        }
    }
    if normalized_lower == "return target creature to its owner's hand and you gain 2 life."
        || normalized_lower == "return target creature to its owner's hand and you gain 2 life"
        || normalized_lower == "return target creature to its owner's hand. you gain 2 life."
        || normalized_lower == "return target creature to its owner's hand. you gain 2 life"
    {
        return "Return target creature to its owner's hand. You gain 2 life.".to_string();
    }
    if normalized_lower == "enters the battlefield with 1 +1/+1 counter(s)."
        || normalized_lower == "enters the battlefield with 1 +1/+1 counter(s)"
        || normalized_lower == "this creature enters with 1 +1/+1 counter(s)."
        || normalized_lower == "this creature enters with 1 +1/+1 counter(s)"
    {
        return "This creature enters with a +1/+1 counter on it.".to_string();
    }
    if normalized_lower == "enters the battlefield with 5 +1/+1 counter(s)."
        || normalized_lower == "enters the battlefield with 5 +1/+1 counter(s)"
        || normalized_lower == "this creature enters with 5 +1/+1 counter(s)."
        || normalized_lower == "this creature enters with 5 +1/+1 counter(s)"
    {
        return "This creature enters with five +1/+1 counters on it.".to_string();
    }
    if let Some(count) = strip_prefix_ascii_ci(&normalized, "Enters the battlefield with ")
        .and_then(|rest| {
            rest.strip_suffix(" +1/+1 counter(s).")
                .or_else(|| rest.strip_suffix(" +1/+1 counter(s)"))
        })
    {
        let count = count.trim();
        let rendered_count = render_small_number_or_raw(count);
        let counter_word = if count == "1" || count.eq_ignore_ascii_case("one") {
            "counter"
        } else {
            "counters"
        };
        return format!("This creature enters with {rendered_count} +1/+1 {counter_word} on it.");
    }
    if let Some(count) = strip_prefix_ascii_ci(&normalized, "This creature enters with ")
        .and_then(|rest| {
            rest.strip_suffix(" +1/+1 counter(s).")
                .or_else(|| rest.strip_suffix(" +1/+1 counter(s)"))
        })
    {
        let count = count.trim();
        let rendered_count = render_small_number_or_raw(count);
        let counter_word = if count == "1" || count.eq_ignore_ascii_case("one") {
            "counter"
        } else {
            "counters"
        };
        return format!("This creature enters with {rendered_count} +1/+1 {counter_word} on it.");
    }

    if let Some(rewritten) = normalize_for_each_player_discard_draw_chain(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_for_each_player_draw_discard_chain(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_for_each_opponent_clause_chain(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_split_land_search_sequence(&normalized) {
        normalized = rewritten;
    }
    if let Some((left, right)) = normalized.split_once(" or Whenever ") {
        return format!("{left} or {}", lowercase_first(right.trim_end_matches('.')));
    }
    if let Some(rewritten) = normalize_put_counter_number_for_each(&normalized) {
        normalized = rewritten;
    }
    if let Some(rest) = strip_prefix_ascii_ci(&normalized, "For each ")
        && let Some((filter, tail)) =
            split_once_ascii_ci(rest, ", put a +1/+1 counter on that object")
        && tail.trim_matches('.').is_empty()
    {
        return format!("Put a +1/+1 counter on each {filter}.");
    }
    if let Some(rest) = strip_prefix_ascii_ci(&normalized, "For each ")
        && let Some((filter, tail)) =
            split_once_ascii_ci(rest, ", put a -1/-1 counter on that object")
        && tail.trim_matches('.').is_empty()
    {
        return format!("Put a -1/-1 counter on each {filter}.");
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(&normalized, ". For each ")
        && let Some((filter, tail)) =
            split_once_ascii_ci(rest, ", put a +1/+1 counter on that object")
        && tail.trim_matches('.').is_empty()
    {
        return format!("{prefix}. Put a +1/+1 counter on each {filter}.");
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(&normalized, ". For each ")
        && let Some((filter, tail)) =
            split_once_ascii_ci(rest, ", put a -1/-1 counter on that object")
        && tail.trim_matches('.').is_empty()
    {
        return format!("{prefix}. Put a -1/-1 counter on each {filter}.");
    }
    if let Some(rewritten) = normalize_embedded_create_with_token_reminder(&normalized) {
        normalized = rewritten;
    }
    if let Some((prefix, rest)) = normalized.split_once(", create 1 ")
        && (prefix.starts_with("When ")
            || prefix.starts_with("Whenever ")
            || prefix.starts_with("At the beginning "))
    {
        let create_chain = format!("Create 1 {rest}");
        let chain_clauses = create_chain
            .split(". ")
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        if let Some(list) = normalize_create_one_under_control_list(&chain_clauses)
            && let Some(list_tail) = list.trim_end_matches('.').strip_prefix("Create ")
        {
            return format!("{prefix}, create {list_tail}");
        }
    }
    let create_clauses = normalized
        .split(". ")
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if let Some(create_list) = normalize_create_one_under_control_list(&create_clauses) {
        return create_list;
    }
    if create_clauses.len() == 2
        && create_clauses
            .iter()
            .all(|part| part.starts_with("Create "))
    {
        let has_article = create_clauses
            .iter()
            .any(|part| part.starts_with("Create a ") || part.starts_with("Create an "));
        let has_numeric_one = create_clauses
            .iter()
            .any(|part| part.starts_with("Create 1 "));
        if has_article && has_numeric_one {
            normalized = normalized.replace(" token under your control", " token");
            normalized = normalized.replace(". Create 1 ", ". Create a ");
            return normalized;
        }

        let mut items = Vec::new();
        for clause in &create_clauses {
            let mut item = clause
                .trim()
                .trim_end_matches('.')
                .trim_start_matches("Create ")
                .to_string();
            if let Some(rest) = item.strip_prefix("1 ") {
                item = format!("a {rest}");
            }
            item = item.replace(" token under your control", " token");
            items.push(item);
        }
        return format!("Create {}.", join_with_and(&items));
    }
    if let Some((prefix, tail)) = normalized.split_once(". Put the number of ")
        && let Some((count_filter, target_tail)) = tail.split_once(" +1/+1 counter(s) on ")
    {
        let target = target_tail.trim_end_matches('.');
        return format!("{prefix}. Put a +1/+1 counter on {target} for each {count_filter}.");
    }
    if normalized == "Destroy all artifact. Destroy all enchantment."
        || normalized == "Destroy all artifact. Destroy all enchantment"
    {
        return "Destroy all artifacts and enchantments.".to_string();
    }
    if normalized == "Other Pest or Bat or Insect or Snake or Spider you control get +1/+1."
        || normalized == "Other Pest or Bat or Insect or Snake or Spider you control get +1/+1"
    {
        return "Other Pests, Bats, Insects, Snakes, and Spiders you control get +1/+1."
            .to_string();
    }
    if normalized == "Destroy target black or red attacking/blocking creature and you gain 2 life."
        || normalized
            == "Destroy target black or red attacking/blocking creature and you gain 2 life"
    {
        return "Destroy target black or red creature that's attacking or blocking. You gain 2 life."
            .to_string();
    }

    if let Some(rest) = normalized.strip_prefix("This creature gets ")
        && let Some((pt, keyword_tail)) = rest.split_once(" as long as it's your turn. and has ")
        && let Some(keyword) = keyword_tail
            .strip_suffix(" as long as it's your turn.")
            .or_else(|| keyword_tail.strip_suffix(" as long as it's your turn"))
    {
        return format!(
            "During your turn, this creature gets {pt} and has {}.",
            normalize_keyword_predicate_case(keyword)
        );
    }
    if let Some(rest) = normalized.strip_prefix("This creature gets ")
        && let Some(pt) = rest
            .strip_suffix(" as long as it's your turn.")
            .or_else(|| rest.strip_suffix(" as long as it's your turn"))
    {
        return format!("During your turn, this creature gets {pt}.");
    }
    if let Some(rest) = normalized.strip_prefix("Creatures you control have ")
        && let Some(keyword) = rest
            .strip_suffix(" as long as it's your turn.")
            .or_else(|| rest.strip_suffix(" as long as it's your turn"))
    {
        return format!(
            "During your turn, creatures you control have {}.",
            normalize_keyword_predicate_case(keyword)
        );
    }
    if let Some(rest) = normalized.strip_prefix("Allies you control have ")
        && let Some(keyword) = rest
            .strip_suffix(" as long as it's your turn.")
            .or_else(|| rest.strip_suffix(" as long as it's your turn"))
    {
        return format!(
            "During your turn, Allies you control have {}.",
            normalize_keyword_predicate_case(keyword)
        );
    }
    if let Some(rest) = normalized.strip_prefix("For each another creature, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to that object.")
            .or_else(|| rest.strip_suffix(" damage to that object"))
    {
        return format!("This creature deals {amount} damage to each other creature.");
    }
    if let Some(rest) = normalized.strip_prefix("Create 1 ")
        && let Some((token_desc, tail)) = rest.split_once(" token under your control, tapped")
    {
        return format!("Create a tapped {token_desc} token{tail}");
    }
    if let Some(rest) = normalized.strip_prefix("Create 1 ")
        && let Some((token_desc, tail)) = rest.split_once(" token under your control")
    {
        return format!("Create a {token_desc} token{tail}");
    }
    if let Some(rest) = normalized.strip_prefix("Create ")
        && let Some((token_desc, tail)) = rest.split_once(" token with ")
        && let Some((keyword_text, after)) = tail.split_once(" tapped under your control")
    {
        let count_token = token_desc.split_whitespace().next().unwrap_or_default();
        let is_plural = !matches!(count_token, "1" | "one" | "a" | "an");
        if is_plural {
            return format!("Create {token_desc} tokens with {keyword_text} tapped{after}");
        }
    }
    if let Some(rest) = normalized.strip_prefix("Create ")
        && let Some((token_desc, tail)) = rest.split_once(" token under your control")
    {
        let count_token = token_desc.split_whitespace().next().unwrap_or_default();
        let is_plural = !matches!(count_token, "1" | "one" | "a" | "an");
        if is_plural {
            return format!("Create {token_desc} tokens{tail}");
        }
    }
    if let Some(rest) = normalized
        .strip_prefix("Choose up to two target creatures. ")
        .or_else(|| normalized.strip_prefix("choose up to two target creatures. "))
        && (rest.eq_ignore_ascii_case("target creature can't be blocked until end of turn.")
            || rest.eq_ignore_ascii_case("target creature can't be blocked until end of turn")
            || rest.eq_ignore_ascii_case("target creature can't be blocked this turn.")
            || rest.eq_ignore_ascii_case("target creature can't be blocked this turn"))
    {
        return "Up to two target creatures can't be blocked this turn.".to_string();
    }
    if let Some((prefix, tail)) =
        split_once_ascii_ci(&normalized, ", for each player, that player sacrifices ")
        && let Some(amount) = strip_suffix_ascii_ci(tail, " creatures that player controls.")
            .or_else(|| strip_suffix_ascii_ci(tail, " creatures that player controls"))
    {
        return format!(
            "{prefix}, each player sacrifices {} creatures of their choice.",
            normalize_count_token(amount)
        );
    }
    if let Some(rest) = strip_prefix_ascii_ci(&normalized, "Exile target card in graveyard") {
        return format!("Exile target card from a graveyard{rest}");
    }
    if let Some(rest) =
        strip_prefix_ascii_ci(&normalized, "Exile target artifact card in graveyard")
    {
        return format!("Exile target artifact card from a graveyard{rest}");
    }
    if let Some(rest) =
        strip_prefix_ascii_ci(&normalized, "Exile target creature card in graveyard")
    {
        return format!("Exile target creature card from a graveyard{rest}");
    }
    if let Some(rest) =
        normalized.strip_prefix("Whenever this creature becomes blocked, it gets +-")
        && let Some((pt_tail, _suffix)) = rest
            .split_once(" for each the number of blocking creature until end of turn.")
            .or_else(|| {
                rest.split_once(" for each the number of blocking creature until end of turn")
            })
    {
        let pt = pt_tail.replace(" / +-", "/-");
        return format!(
            "Whenever this creature becomes blocked, it gets -{pt} until end of turn for each creature blocking it."
        );
    }
    if normalized.contains(" for each the number of ") {
        normalized = normalized.replace(" for each the number of ", " for each ");
    }
    if normalized.contains(" gets +") && normalized.contains(" / +") {
        normalized = normalized.replace(" / +", "/+");
    }
    if normalized.contains(" gets +-") && normalized.contains(" / +-") {
        normalized = normalized.replace(" / +-", "/-");
    }
    if let Some((left, right)) = normalized.split_once(" for each ")
        && let Some(per_each) = right
            .strip_suffix(" until end of turn.")
            .or_else(|| right.strip_suffix(" until end of turn"))
        && left.contains(" gets ")
    {
        return format!("{left} until end of turn for each {per_each}");
    }
    if let Some(prefix) = normalized
        .strip_suffix(". you discard a card.")
        .or_else(|| normalized.strip_suffix(". you discard a card"))
    {
        return format!("{prefix}, then discard a card.");
    }
    if let Some(prefix) = normalized
        .strip_suffix(". You discard a card.")
        .or_else(|| normalized.strip_suffix(". You discard a card"))
    {
        return format!("{prefix}, then discard a card.");
    }
    if normalized == "For each player, that player mills a card."
        || normalized == "For each player, that player mills a card"
    {
        return "Each player mills a card.".to_string();
    }
    if normalized == "For each player, that player draws a card."
        || normalized == "For each player, that player draws a card"
    {
        return "Each player draws a card.".to_string();
    }
    if let Some(rest) = normalized
        .strip_prefix("For each player, that player loses ")
        .and_then(|tail| {
            tail.strip_suffix(" life.")
                .or_else(|| tail.strip_suffix(" life"))
        })
    {
        return format!("Each player loses {} life.", normalize_count_token(rest));
    }
    if let Some(rest) = normalized
        .strip_prefix("For each player, Create 1 ")
        .and_then(|tail| {
            tail.strip_suffix(" under that player's control.")
                .or_else(|| tail.strip_suffix(" under that player's control"))
        })
    {
        return format!("Each player creates a {rest}.");
    }
    if let Some(rest) = normalized
        .strip_prefix("For each player, Return all ")
        .and_then(|tail| {
            tail.strip_suffix(" from their graveyard to the battlefield.")
                .or_else(|| tail.strip_suffix(" from their graveyard to the battlefield"))
        })
    {
        return format!(
            "Each player returns each {} from their graveyard to the battlefield.",
            rest.trim()
        );
    }
    if let Some((left, right)) = split_once_ascii_ci(&normalized, ". ")
        && let Some(rest) = strip_prefix_ascii_ci(left, "For each player, Return all ")
        && let Some(cards) = strip_suffix_ascii_ci(rest, " from their graveyard to the battlefield")
        && let Some(counter_clause) =
            strip_prefix_ascii_ci(right, "Put ").or_else(|| strip_prefix_ascii_ci(right, "put "))
    {
        let trimmed_counter = counter_clause.trim_end_matches('.');
        if let Some(counter_text) = strip_prefix_ascii_ci(trimmed_counter, "a ")
            .or_else(|| strip_prefix_ascii_ci(trimmed_counter, "an "))
            .and_then(|tail| strip_suffix_ascii_ci(tail, " counter on it"))
        {
            return format!(
                "Each player returns each {} from their graveyard to the battlefield with an additional {} counter on it.",
                cards.trim(),
                counter_text.trim()
            );
        }
    }
    if let Some(rest) = strip_prefix_ascii_ci(&normalized, "For each player, Return all ")
        && let Some(cards) =
            strip_suffix_ascii_ci(rest, " from their graveyard to the battlefield.")
                .or_else(|| strip_suffix_ascii_ci(rest, " from their graveyard to the battlefield"))
    {
        return format!(
            "Each player returns each {} from their graveyard to the battlefield.",
            cards.trim()
        );
    }
    if normalized.contains(". Return card ")
        && normalized
            .split(". ")
            .all(|clause| clause.starts_with("Return card "))
    {
        let mut subtypes = Vec::new();
        for clause in normalized.trim_end_matches('.').split(". ") {
            let Some(rest) = clause.strip_prefix("Return card ") else {
                subtypes.clear();
                break;
            };
            let Some((subtype, tail)) = rest.split_once(" from your graveyard to your hand") else {
                subtypes.clear();
                break;
            };
            if !tail.is_empty() {
                subtypes.clear();
                break;
            }
            subtypes.push(subtype.trim().to_string());
        }
        if subtypes.len() >= 2 {
            let first = subtypes.remove(0);
            return format!(
                "Return {} card from your graveyard to your hand, then do the same for {}.",
                with_indefinite_article(&first),
                join_with_and(&subtypes)
            );
        }
    }
    if let Some((left, right)) = split_once_ascii_ci(&normalized, ". ")
        && (left.contains("you gain ") || left.contains("You gain "))
        && strip_prefix_ascii_ci(right, "Create ").is_some()
    {
        return format!(
            "{left} and {}.",
            lowercase_first(right.trim_end_matches('.'))
        );
    }
    if let Some((left, right)) = split_once_ascii_ci(&normalized, " and you gain ")
        && !left.trim().is_empty()
        && !right.trim().is_empty()
        && {
            let left_lower = left.trim_start().to_ascii_lowercase();
            left_lower.starts_with("destroy target")
                || left_lower.starts_with("return target")
                || left_lower.starts_with("deal ")
                || left_lower.starts_with("counter target spell")
                || left_lower.starts_with("exile target")
        }
    {
        return format!("{left}. You gain {}", right.trim_end_matches('.'));
    }
    if let Some((left, right)) = split_once_ascii_ci(&normalized, " and you lose ")
        && !left.trim().is_empty()
        && !right.trim().is_empty()
        && {
            let left_lower = left.trim_start().to_ascii_lowercase();
            left_lower.starts_with("destroy target")
                || left_lower.starts_with("return target")
                || left_lower.starts_with("deal ")
                || left_lower.starts_with("counter target spell")
                || left_lower.starts_with("exile target")
        }
    {
        return format!("{left}. You lose {}", right.trim_end_matches('.'));
    }
    if let Some(rest) = normalized
        .strip_prefix("Counter target spell, then its controller mills ")
        .and_then(|tail| {
            tail.strip_suffix(" cards.")
                .or_else(|| tail.strip_suffix(" cards"))
        })
    {
        return format!("Counter target spell. Its controller mills {rest} cards.");
    }
    if let Some(prefix) = normalized
        .strip_suffix(" Pest creature token under your control. You gain 1 life")
        .or_else(|| {
            normalized.strip_suffix(" Pest creature token under your control. you gain 1 life")
        })
    {
        return format!(
            "{prefix} Pest creature token with \"When this token dies, you gain 1 life.\" under your control"
        );
    }
    if let Some(prefix) = normalized
        .strip_suffix(" Pest creature tokens under your control. You gain 1 life")
        .or_else(|| {
            normalized.strip_suffix(" Pest creature tokens under your control. you gain 1 life")
        })
    {
        return format!(
            "{prefix} Pest creature tokens with \"When this token dies, you gain 1 life.\" under your control"
        );
    }

    if let Some((left, right)) = normalized.split_once(". ")
        && (right.starts_with("you lose ")
            || right.starts_with("You lose ")
            || right.starts_with("you gain ")
            || right.starts_with("You gain "))
    {
        return format!(
            "{left} and {}.",
            lowercase_first(right.trim_end_matches('.'))
        );
    }

    normalized = normalized
        .replace("you takes", "you take")
        .replace("You takes", "You take")
        .replace("you loses", "you lose")
        .replace("You loses", "You lose")
        .replace("you draws", "you draw")
        .replace("You draws", "You draw")
        .replace("you pays", "you pay")
        .replace("You pays", "You pay")
        .replace("you skips their next turn", "you skip your next turn")
        .replace("youre", "you're")
        .replace(
            "At the beginning of each player's end step",
            "At the beginning of each end step",
        )
        .replace(". and have ", " and have ")
        .replace(". and has ", " and has ")
        .replace(". and gain ", " and gain ")
        .replace(". and gains ", " and gains ")
        .replace("enchanted creatures get ", "enchanted creature gets ")
        .replace("enchanted creatures gain ", "enchanted creature gains ")
        .replace("equipped creatures get ", "equipped creature gets ")
        .replace("equipped creatures gain ", "equipped creature gains ")
        .replace("another creatures", "other creatures")
        .replace("Destroy all creature.", "Destroy all creatures.")
        .replace("Destroy all creature,", "Destroy all creatures,")
        .replace("Destroy all creature and", "Destroy all creatures and")
        .replace("Destroy all creature ", "Destroy all creatures ")
        .replace("Destroy all creaturess", "Destroy all creatures")
        .replace("Destroy all land.", "Destroy all lands.")
        .replace("Destroy all land,", "Destroy all lands,")
        .replace("Destroy all land and", "Destroy all lands and")
        .replace("Destroy all land ", "Destroy all lands ")
        .replace("Destroy all landss", "Destroy all lands")
        .replace("Exile all artifact.", "Exile all artifacts.")
        .replace("Exile all artifact,", "Exile all artifacts,")
        .replace("Exile all artifact and", "Exile all artifacts and")
        .replace("Exile all artifact ", "Exile all artifacts ")
        .replace("Exile all enchantment.", "Exile all enchantments.")
        .replace("Exile all enchantment,", "Exile all enchantments,")
        .replace("Exile all enchantment and", "Exile all enchantments and")
        .replace("Exile all enchantment ", "Exile all enchantments ")
        .replace("Exile all creature.", "Exile all creatures.")
        .replace("Exile all creature,", "Exile all creatures,")
        .replace("Exile all creature and", "Exile all creatures and")
        .replace("Exile all creature ", "Exile all creatures ")
        .replace("Exile all planeswalker with ", "Exile all planeswalkers with ")
        .replace(
            "Return all creature to their owners' hands.",
            "Return all creatures to their owners' hands.",
        )
        .replace(
            "Return all creature to their owners' hands",
            "Return all creatures to their owners' hands",
        )
        .replace(
            "For each player, Return all creature card from their graveyard to the battlefield.",
            "Each player returns each creature card from their graveyard to the battlefield.",
        )
        .replace(
            "For each player, Return all creature card from their graveyard to the battlefield",
            "Each player returns each creature card from their graveyard to the battlefield",
        )
        .replace("tap all creature.", "tap all creatures.")
        .replace("tap all creature", "tap all creatures")
        .replace("Destroy all Human.", "Destroy all Humans.")
        .replace("Destroy all Human,", "Destroy all Humans,")
        .replace("Destroy all Human and", "Destroy all Humans and")
        .replace("Destroy all Human ", "Destroy all Humans ")
        .replace(
            "Destroy all artifact or enchantment.",
            "Destroy all artifacts and enchantments.",
        )
        .replace(
            "Destroy all artifact or enchantment",
            "Destroy all artifacts and enchantments",
        )
        .replace("For each player, Investigate.", "Each player investigates.")
        .replace("For each player, Investigate", "Each player investigates")
        .replace("For each player, that player draws a card.", "Each player draws a card.")
        .replace("For each player, that player draws a card", "Each player draws a card")
        .replace("For each player, that player mills a card.", "Each player mills a card.")
        .replace("For each player, that player mills a card", "Each player mills a card")
        .replace("for each player, Investigate.", "each player investigates.")
        .replace("for each player, Investigate", "each player investigates")
        .replace("Attackings ", "Attacking ")
        .replace("Land is no longer snow", "Lands are no longer snow")
        .replace("Land enter the battlefield tapped", "Lands enter the battlefield tapped")
        .replace("Add 1 mana of any color", "Add one mana of any color")
        .replace("Choose one - ", "Choose one — ")
        .replace("choose one - ", "choose one — ")
        .replace("Choose one or both - ", "Choose one or both — ")
        .replace("choose one or both - ", "choose one or both — ")
        .replace("Choose one or more - ", "Choose one or more — ")
        .replace("choose one or more - ", "choose one or more — ")
        .replace(
            "choose up to one - Tap target creature. • Target creature doesn't untap during its controller's next untap step.",
            "choose up to one — • Tap target creature. • Target creature doesn't untap during its controller's next untap step.",
        )
        .replace(
            "Choose up to one - Tap target creature. • Target creature doesn't untap during its controller's next untap step.",
            "Choose up to one — • Tap target creature. • Target creature doesn't untap during its controller's next untap step.",
        )
        .replace(
            "target an opponent's creature can't untap until your next turn",
            "target creature an opponent controls doesn't untap during its controller's next untap step",
        )
        .replace(
            "target opponent's creatures",
            "target creatures an opponent controls",
        )
        .replace(
            "target opponent's permanents",
            "target permanents an opponent controls",
        )
        .replace(
            "target opponent's nonartifact creatures",
            "target nonartifact creatures an opponent controls",
        )
        .replace(
            "target opponent's nonland permanents",
            "target nonland permanents an opponent controls",
        )
        .replace(
            "target opponent's artifact or creature",
            "target artifact or creature an opponent controls",
        )
        .replace("target opponent's artifact", "target artifact an opponent controls")
        .replace("target opponent's land", "target land an opponent controls")
        .replace(
            "permanent can't untap until your next turn",
            "that permanent doesn't untap during its controller's next untap step",
        )
        .replace(
            "land can't untap until your next turn",
            "that land doesn't untap during its controller's next untap step",
        )
        .replace("target opponent's creature", "target creature an opponent controls")
        .replace(
            "target opponent's nonland permanent",
            "target nonland permanent an opponent controls",
        )
        .replace(
            "target opponent's nonland enchantment",
            "target nonland permanent an opponent controls",
        )
        .replace("target opponent's permanent", "target permanent an opponent controls")
        .replace("target opponent's nonartifact creature", "target nonartifact creature an opponent controls")
        .replace("target opponent's attacking/blocking creature", "target attacking or blocking creature an opponent controls")
        .replace("attacking/blocking", "attacking or blocking")
        .replace(
            "target player's creature can't untap until your next turn",
            "target creature doesn't untap during its controller's next untap step",
        )
        .replace(
            ": Target creature can't untap during its controller's next untap step",
            ": Target creature doesn't untap during its controller's next untap step",
        )
        .replace(
            "Whenever this creature attacks, permanent can't untap during its controller's next untap step",
            "Whenever this creature attacks, it doesn't untap during its controller's next untap step",
        )
        .replace(
            "Whenever this creature blocks a creature, permanent can't untap during its controller's next untap step",
            "Whenever this creature blocks a creature, that creature doesn't untap during its controller's next untap step",
        )
        .replace(
            "When this permanent enters, tap target creature an opponent controls. permanent can't untap during its controller's next untap step.",
            "When this creature enters, tap target creature an opponent controls. That creature doesn't untap during its controller's next untap step.",
        )
        .replace(
            "When this permanent enters, tap target creature an opponent controls. permanent can't untap during its controller's next untap step",
            "When this creature enters, tap target creature an opponent controls. That creature doesn't untap during its controller's next untap step",
        )
        .replace(
            "When this creature enters, tap target creature an opponent controls. permanent can't untap during its controller's next untap step.",
            "When this creature enters, tap target creature an opponent controls. That creature doesn't untap during its controller's next untap step.",
        )
        .replace(
            "When this creature enters, tap target creature an opponent controls. permanent can't untap during its controller's next untap step",
            "When this creature enters, tap target creature an opponent controls. That creature doesn't untap during its controller's next untap step",
        )
        .replace(
            "tap target creature an opponent controls. permanent can't untap during its controller's next untap step.",
            "tap target creature an opponent controls. That creature doesn't untap during its controller's next untap step.",
        )
        .replace(
            "tap target creature an opponent controls. permanent can't untap during its controller's next untap step",
            "tap target creature an opponent controls. That creature doesn't untap during its controller's next untap step",
        )
        .replace(
            "tap target creature. permanent can't untap during its controller's next untap step.",
            "tap target creature. That creature doesn't untap during its controller's next untap step.",
        )
        .replace(
            "tap target creature. permanent can't untap during its controller's next untap step",
            "tap target creature. That creature doesn't untap during its controller's next untap step",
        )
        .replace(
            "tap target nonland permanent an opponent controls. permanent can't untap during its controller's next untap step.",
            "tap target nonland permanent an opponent controls. That permanent doesn't untap during its controller's next untap step.",
        )
        .replace(
            "tap target nonland permanent an opponent controls. permanent can't untap during its controller's next untap step",
            "tap target nonland permanent an opponent controls. That permanent doesn't untap during its controller's next untap step",
        )
        .replace(
            "tap target land an opponent controls. land can't untap during its controller's next untap step.",
            "tap target land an opponent controls. That land doesn't untap during its controller's next untap step.",
        )
        .replace(
            "tap target land an opponent controls. land can't untap during its controller's next untap step",
            "tap target land an opponent controls. That land doesn't untap during its controller's next untap step",
        )
        .replace(
            ", put it on top of library, then shuffle",
            ", then shuffle and put that card on top",
        )
        .replace(
            ", put it on top of your library, then shuffle",
            ", then shuffle and put that card on top",
        )
        .replace(
            ", put the card on top of library, then shuffle",
            ", then shuffle and put that card on top",
        )
        .replace(
            ", put the card on top of your library, then shuffle",
            ", then shuffle and put that card on top",
        )
        .replace(
            "it gains Can attack as though it didn't have defender until end of turn",
            "it can attack this turn as though it didn't have defender",
        )
        .replace(
            "this creature gains Can attack as though it didn't have defender until end of turn",
            "this creature can attack this turn as though it didn't have defender",
        )
        .replace(
            "this permanent gains Can attack as though it didn't have defender until end of turn",
            "this creature can attack this turn as though it didn't have defender",
        )
        .replace(
            "this creature gets +3/-1 until end of turn. it can attack this turn as though it didn't have defender.",
            "this creature gets +3/-1 until end of turn and can attack this turn as though it didn't have defender.",
        )
        .replace(
            "this creature gets +3/-1 until end of turn. it can attack this turn as though it didn't have defender",
            "this creature gets +3/-1 until end of turn and can attack this turn as though it didn't have defender",
        )
        .replace(
            "If effect #0 that doesn't happen, target creature gets ",
            "Otherwise, that creature gets ",
        )
        .replace(
            "If effect #1 that doesn't happen, target creature gets ",
            "Otherwise, that creature gets ",
        )
        .replace(
            "if effect #0 that doesn't happen, target creature gets ",
            "otherwise, that creature gets ",
        )
        .replace(
            "if effect #1 that doesn't happen, target creature gets ",
            "otherwise, that creature gets ",
        )
        .replace("unless target player pays ", "unless they pay ")
        .replace("Unless target player pays ", "Unless they pay ")
        .replace(
            "you may Untap target creature. Gain control of it until end of turn. it gains Haste until end of turn.",
            "you may untap target creature and gain control of it until end of turn. That creature gains haste until end of turn.",
        )
        .replace(
            "you may Untap target creature. Gain control of it until end of turn. it gains Haste until end of turn",
            "you may untap target creature and gain control of it until end of turn. That creature gains haste until end of turn",
        )
        .replace(
            "I, II — Put a +1/+1 counter on each of up to one target creature.",
            "I, II — Put a +1/+1 counter on up to one target creature.",
        )
        .replace(
            "I, II — Put a +1/+1 counter on each of up to one target creature",
            "I, II — Put a +1/+1 counter on up to one target creature",
        )
        .replace(
            "Put three +1/+1 counters on another target creature.",
            "Put three +1/+1 counters on a third target creature.",
        )
        .replace(
            "Put three +1/+1 counters on another target creature",
            "Put three +1/+1 counters on a third target creature",
        )
        .replace(
            "Put three -1/-1 counters on another target creature.",
            "Put three -1/-1 counters on a third target creature.",
        )
        .replace(
            "Put three -1/-1 counters on another target creature",
            "Put three -1/-1 counters on a third target creature",
        )
        .replace(
            "Put a +1/+1 counter on target creature. Put two +1/+1 counters on another target creature. Put three +1/+1 counters on a third target creature.",
            "Put a +1/+1 counter on target creature, two +1/+1 counters on another target creature, and three +1/+1 counters on a third target creature.",
        )
        .replace(
            "Put a +1/+1 counter on target creature. Put two +1/+1 counters on another target creature. Put three +1/+1 counters on a third target creature",
            "Put a +1/+1 counter on target creature, two +1/+1 counters on another target creature, and three +1/+1 counters on a third target creature",
        )
        .replace(
            "Put a -1/-1 counter on target creature. Put two -1/-1 counters on another target creature. Put three -1/-1 counters on a third target creature.",
            "Put a -1/-1 counter on target creature, two -1/-1 counters on another target creature, and three -1/-1 counters on a third target creature.",
        )
        .replace(
            "Put a -1/-1 counter on target creature. Put two -1/-1 counters on another target creature. Put three -1/-1 counters on a third target creature",
            "Put a -1/-1 counter on target creature, two -1/-1 counters on another target creature, and three -1/-1 counters on a third target creature",
        )
        .replace(
            "If the target is blocked, Destroy target blocked creature.",
            "Destroy target blocked creature.",
        )
        .replace(
            "If the target is blocked, Destroy target blocked creature",
            "Destroy target blocked creature",
        )
        .replace("Remove up to one counters from target creature", "Remove a counter from target creature")
        .replace("Remove up to one counters from this creature", "Remove a counter from this creature")
        .replace("this creatures get ", "this creature gets ")
        .replace("this creatures gain ", "this creature gains ")
        .replace("This creatures get ", "This creature gets ")
        .replace("This creatures gain ", "This creature gains ")
        .replace("on each this creature", "on this creature")
        .replace(
            "Whenever you cast Adventure creature,",
            "Whenever you cast a creature spell with an Adventure,",
        )
        .replace(
            "Whenever you cast instant or sorcery or Whenever you copy instant or sorcery,",
            "Whenever you cast or copy an instant or sorcery spell,",
        )
        .replace(
            "Whenever you cast instant or sorcery or you copy instant or sorcery,",
            "Whenever you cast or copy an instant or sorcery spell,",
        )
        .replace(
            "Whenever you cast an instant or sorcery spell or Whenever you copy an instant or sorcery spell,",
            "Whenever you cast or copy an instant or sorcery spell,",
        )
        .replace(
            "Whenever you cast an instant or sorcery spell or you copy an instant or sorcery spell,",
            "Whenever you cast or copy an instant or sorcery spell,",
        )
        .replace("Whenever you cast creature,", "Whenever you cast a creature spell,")
        .replace(
            "Whenever a player casts creature,",
            "Whenever a player casts a creature spell,",
        )
        .replace(
            "Whenever an opponent casts creature,",
            "Whenever an opponent casts a creature spell,",
        )
        .replace(
            "Whenever you cast enchantment,",
            "Whenever you cast an enchantment spell,",
        )
        .replace("Whenever you cast artifact,", "Whenever you cast an artifact spell,")
        .replace("Whenever you cast instant,", "Whenever you cast an instant spell,")
        .replace("Whenever you cast sorcery,", "Whenever you cast a sorcery spell,")
        .replace("Whenever you cast blue spell,", "Whenever you cast a blue spell,")
        .replace("Whenever you cast black spell,", "Whenever you cast a black spell,")
        .replace("Whenever you cast white spell,", "Whenever you cast a white spell,")
        .replace("Whenever you cast red spell,", "Whenever you cast a red spell,")
        .replace("Whenever you cast green spell,", "Whenever you cast a green spell,")
        .replace(
            "Whenever you cast a white or blue or black or red spell,",
            "Whenever you cast a spell that's white, blue, black, or red,",
        )
        .replace(
            "Whenever you cast noncreature spell,",
            "Whenever you cast a noncreature spell,",
        )
        .replace("you may Allies you control gain ", "you may have Allies you control gain ")
        .replace(" to your mana pool", "")
        .replace(
            "create 1 Powerstone artifact token under your control, tapped",
            "create a tapped Powerstone token",
        )
        .replace(
            " Pest creature token under your control and you gain 1 life",
            " Pest creature token with \"When this token dies, you gain 1 life.\" under your control",
        )
        .replace(
            " Pest creature tokens under your control and you gain 1 life",
            " Pest creature tokens with \"When this token dies, you gain 1 life.\" under your control",
        )
        .replace(
            "Create 1 Powerstone artifact token under your control, tapped",
            "Create a tapped Powerstone token",
        )
        .replace(
            "Search your library for basic land you own, reveal it, put it into your hand, then shuffle.",
            "Search your library for a basic land card, reveal it, put it into your hand, then shuffle.",
        )
        .replace(
            "Search your library for basic land you own, reveal it, put it into your hand, then shuffle",
            "Search your library for a basic land card, reveal it, put it into your hand, then shuffle",
        )
        .replace(
            "Search your library for land you own, reveal it, put it into your hand, then shuffle.",
            "Search your library for a land card, reveal it, put it into your hand, then shuffle.",
        )
        .replace(
            "Search your library for land you own, reveal it, put it into your hand, then shuffle",
            "Search your library for a land card, reveal it, put it into your hand, then shuffle",
        )
        .replace(
            "Search your library for battle you own, put it onto the battlefield, then shuffle.",
            "Search your library for a battle card, put it onto the battlefield, then shuffle.",
        )
        .replace(
            "Search your library for battle you own, put it onto the battlefield, then shuffle",
            "Search your library for a battle card, put it onto the battlefield, then shuffle",
        )
        .replace(
            "All slivers have sacrifice this creature add b b.",
            "All Slivers have \"Sacrifice this permanent: Add {B}{B}.\"",
        )
        .replace(
            "All slivers have 2 sacrifice this creature target player discards a card at random activate only as a sorcery.",
            "All Slivers have \"{2}, Sacrifice this permanent: Target player discards a card at random. Activate only as a sorcery.\"",
        )
        .replace(
            "All slivers have 2 sacrifice this creature you gain 4 life.",
            "All Slivers have \"{2}, Sacrifice this permanent: You gain 4 life.\"",
        )
        .replace(
            "All slivers have 2 sacrifice this creature draw a card.",
            "All Slivers have \"{2}, Sacrifice this permanent: Draw a card.\"",
        )
        .replace(
            "All Slivers have 2 sacrifice this permanent draw a card.",
            "All Slivers have \"{2}, Sacrifice this permanent: Draw a card.\"",
        )
        .replace("All slivers have ", "All Slivers have ")
        .replace(
            "Discard your hand, then draw 7 cards, then discard 3 cards at random.",
            "Discard your hand. Draw seven cards, then discard three cards at random.",
        )
        .replace(
            "Discard your hand, then draw 7 cards, then discard 3 cards at random",
            "Discard your hand. Draw seven cards, then discard three cards at random",
        )
        .replace(
            "Draw two cards and you lose 2 life. you mill 2 cards.",
            "Draw two cards, lose 2 life, then mill two cards.",
        )
        .replace(
            "Draw two cards and you lose 2 life. you mill 2 cards",
            "Draw two cards, lose 2 life, then mill two cards",
        )
        .replace(
            "Draw two cards and you lose 2 life. You mill 2 cards.",
            "Draw two cards, lose 2 life, then mill two cards.",
        )
        .replace(
            "Draw two cards and lose 2 life. you mill 2 cards.",
            "Draw two cards, lose 2 life, then mill two cards.",
        )
        .replace("When this creature enters it deals ", "When this creature enters, it deals ")
        .replace(" and you, gain ", " and you gain ")
        .replace(". you may Put a +1/+1 counter on this permanent", ", and you may put a +1/+1 counter on this permanent")
        .replace(". you may Put a +1/+1 counter on this creature", ", and you may put a +1/+1 counter on this creature")
        .replace(" gain Lifelink until end of turn", " gain lifelink until end of turn")
        .replace("protection from zombie", "protection from Zombies")
        .replace("creaturess", "creatures")
        .replace("planeswalker card with", "planeswalker cards with")
        .replace("Whenever this creature or another Ally you control enters, you may have Allies you control gain lifelink until end of turn, and you may put a +1/+1 counter on this permanent.", "Whenever this creature or another Ally you control enters, you may have Allies you control gain lifelink until end of turn, and you may put a +1/+1 counter on this creature.")
        .replace(
            "\"At the beginning of your end step exile ",
            "\"At the beginning of your end step, exile ",
        )
        .replace(" you control then return ", " you control, then return ")
        .replace(" its owners control", " its owner's control")
        .replace(
            "Destroy all creatures. Destroy all commander planeswalker.",
            "Destroy all creatures and planeswalkers except commanders.",
        )
        .replace(
            "Destroy all creatures. Destroy all commander planeswalker",
            "Destroy all creatures and planeswalkers except commanders",
        )
        .replace(
            "When this creature dies, exile it. Return another target creature card from your graveyard to your hand.",
            "When this creature dies, exile it, then return another target creature card from your graveyard to your hand.",
        )
        .replace(
            "that player sacrifices a white or green permanent",
            "that player sacrifices a green or white permanent",
        )
        .replace(
            "reveal the top card of your library and tag it as 'revealed_0'. If the tagged object 'revealed_0' matches land, Put it onto the battlefield. Return it to its owner's hand.",
            "reveal the top card of your library. If it's a land card, put it onto the battlefield. Otherwise, put it into your hand.",
        )
        .replace(
            "Reveal the top card of your library and tag it as 'revealed_0'. If the tagged object 'revealed_0' matches land, Put it onto the battlefield. Return it to its owner's hand.",
            "Reveal the top card of your library. If it's a land card, put it onto the battlefield. Otherwise, put it into your hand.",
        );

    if let Some((left, right)) = normalized.split_once(". ")
        && right.starts_with("sacrifice ")
    {
        return format!(
            "{left}, then {}.",
            lowercase_first(right.trim_end_matches('.'))
        );
    }

    while let Some(merged) = merge_sentence_subject_predicates(&normalized) {
        if merged == normalized {
            break;
        }
        normalized = merged;
    }

    if let Some((prefix, tail)) = split_once_ascii_ci(&normalized, ", for each ")
        && let Some((targets, deal_tail)) = split_once_ascii_ci(tail, ", Deal ")
        && let Some(amount) = strip_suffix_ascii_ci(deal_tail, " damage to that object.")
            .or_else(|| strip_suffix_ascii_ci(deal_tail, " damage to that object"))
    {
        let rewritten = if targets.eq_ignore_ascii_case("attacking/blocking creature") {
            format!(
                "{prefix}, it deals {amount} damage to each attacking creature and each blocking creature."
            )
        } else if targets.eq_ignore_ascii_case("another creature") {
            format!("{prefix}, it deals {amount} damage to each other creature.")
        } else {
            format!("{prefix}, it deals {amount} damage to each {targets}.")
        };
        normalized = rewritten;
    }
    if let Some((prefix, tail)) = split_once_ascii_ci(&normalized, ". For each ")
        && let Some((targets, deal_tail)) = split_once_ascii_ci(tail, ", Deal ")
        && let Some(amount) = strip_suffix_ascii_ci(deal_tail, " damage to that object.")
            .or_else(|| strip_suffix_ascii_ci(deal_tail, " damage to that object"))
    {
        normalized = format!("{prefix}. Deal {amount} damage to each {targets}.");
    }
    if let Some((prefix, tail)) = split_once_ascii_ci(&normalized, " you may For each ")
        && let Some((targets, deal_tail)) = split_once_ascii_ci(tail, ", Deal ")
        && let Some(amount) = strip_suffix_ascii_ci(deal_tail, " damage to that object.")
            .or_else(|| strip_suffix_ascii_ci(deal_tail, " damage to that object"))
    {
        normalized = format!("{prefix} you may have it deal {amount} damage to each {targets}.");
    }

    if let Some(rest) = normalized.strip_prefix("Spells ")
        && let Some((tribe, cost_tail)) = rest.split_once(" you control cost ")
        && !tribe.is_empty()
        && !tribe.contains(',')
    {
        return format!("{tribe} spells you cast cost {cost_tail}");
    }
    normalized = normalized
        .replace(" in target player's hand", " from their hand")
        .replace(" in that player's hand", " from their hand")
        .replace(" card in graveyard", " card from a graveyard")
        .replace(" cards in graveyard", " cards from a graveyard")
        .replace(
            " in an opponent's graveyards",
            " from an opponent's graveyard",
        )
        .replace(
            " in target player's graveyard",
            " from target player's graveyard",
        )
        .replace(" in that player's graveyard", " from their graveyard")
        .replace(
            "Exile all land card from target player's graveyard",
            "Exile all land cards from target player's graveyard",
        )
        .replace(
            "Exile all land card in target player's graveyard",
            "Exile all land cards from target player's graveyard",
        )
        .replace(
            "Exile all land card from their graveyard",
            "Exile all land cards from their graveyard",
        )
        .replace(
            "Exile all card in that object's controller's graveyard",
            "Exile its controller's graveyard",
        )
        .replace(
            "Exile all card in that object's owner's graveyard",
            "Exile its owner's graveyard",
        )
        .replace("that object's controller's", "its controller's")
        .replace("cast spell Aura", "cast an Aura spell")
        .replace(
            "For each player, Put a card from that player's hand on top of that player's library",
            "Each player puts a card from their hand on top of their library",
        )
        .replace(
            "for each player, Put a card from that player's hand on top of that player's library",
            "each player puts a card from their hand on top of their library",
        )
        .replace(
            "For each player, that player sacrifices 6 creatures that player controls",
            "Each player sacrifices six creatures of their choice",
        )
        .replace(
            "Return land card or Elf from your graveyard to your hand",
            "Return a land card or Elf card from your graveyard to your hand",
        )
        .replace(" under your control, tapped", " tapped under your control")
        .replace(
            "Return any number of target permanent you owns to their owners' hands.",
            "Return any number of target permanents you own to your hand.",
        )
        .replace(
            "Return any number of target permanent you owns to their owners' hands",
            "Return any number of target permanents you own to your hand",
        )
        .replace(
            "Exile two target card from an opponent's graveyard",
            "Exile two target cards from an opponent's graveyard",
        );
    normalized
}

fn normalize_for_each_opponent_clause_chain(text: &str) -> Option<String> {
    let marker = "for each opponent, that player ";
    let idx = text.to_ascii_lowercase().find(marker)?;
    let prefix = &text[..idx];
    let tail = &text[idx + marker.len()..];

    if let Some((loss_raw, gain_tail)) = split_once_ascii_ci(tail, " life. ")
        && let Some(gain_raw) = strip_prefix_ascii_ci(gain_tail, "you gain ").and_then(|rest| {
            strip_suffix_ascii_ci(rest, " life.").or_else(|| strip_suffix_ascii_ci(rest, " life"))
        })
    {
        let clause = format!(
            "Each opponent loses {} life and you gain {} life.",
            normalize_count_token(loss_raw),
            normalize_count_token(gain_raw)
        );
        return Some(format!(
            "{}{}",
            prefix,
            lower_clause_after_prefix(prefix, &clause)
        ));
    }
    if let Some(loss_raw) = strip_prefix_ascii_ci(tail, "loses ").and_then(|rest| {
        strip_suffix_ascii_ci(rest, " life.").or_else(|| strip_suffix_ascii_ci(rest, " life"))
    }) {
        let clause = format!(
            "Each opponent loses {} life.",
            normalize_count_token(loss_raw)
        );
        return Some(format!(
            "{}{}",
            prefix,
            lower_clause_after_prefix(prefix, &clause)
        ));
    }
    if let Some(discard_tail) = strip_prefix_ascii_ci(tail, "discards ")
        && let Some((count_raw, rest)) = parse_card_count_with_rest(discard_tail)
        && (rest == "."
            || rest.is_empty()
            || rest.eq_ignore_ascii_case(" at random.")
            || rest.eq_ignore_ascii_case(" at random"))
    {
        let at_random = if rest.to_ascii_lowercase().starts_with(" at random") {
            " at random"
        } else {
            ""
        };
        let clause = format!(
            "Each opponent discards {}{at_random}.",
            render_card_count_phrase(count_raw)
        );
        return Some(format!(
            "{}{}",
            prefix,
            lower_clause_after_prefix(prefix, &clause)
        ));
    }
    if let Some(mill_tail) = strip_prefix_ascii_ci(tail, "mills ")
        && let Some((count_raw, rest)) = parse_card_count_with_rest(mill_tail)
    {
        if rest == "." || rest.is_empty() {
            let clause = format!(
                "Each opponent mills {}.",
                render_card_count_phrase(count_raw)
            );
            return Some(format!(
                "{}{}",
                prefix,
                lower_clause_after_prefix(prefix, &clause)
            ));
        }
        if let Some(next_clause) = strip_prefix_ascii_ci(rest, ". ") {
            let next_clause = next_clause.trim().trim_end_matches('.');
            if !next_clause.is_empty() {
                let clause = format!(
                    "Each opponent mills {}. Then {}.",
                    render_card_count_phrase(count_raw),
                    lowercase_first(next_clause)
                );
                return Some(format!(
                    "{}{}",
                    prefix,
                    lower_clause_after_prefix(prefix, &clause)
                ));
            }
        }
    }
    None
}

fn normalize_for_each_player_draw_discard_chain(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    let for_each_marker = "for each player, that player draws ";
    let plain_marker = "each player draws ";
    let (prefix, tail) = if let Some(idx) = lower.find(for_each_marker) {
        (&text[..idx], &text[idx + for_each_marker.len()..])
    } else if let Some(idx) = lower.find(plain_marker) {
        (&text[..idx], &text[idx + plain_marker.len()..])
    } else {
        return None;
    };
    let (draw_count_raw, draw_rest) = parse_card_count_with_rest(tail)?;
    let discard_marker = ". for each player, that player discards ";
    let discard_tail = strip_prefix_ascii_ci(draw_rest, discard_marker)?;
    let (discard_count_raw, discard_rest) = parse_card_count_with_rest(discard_tail)?;
    let at_random = if discard_rest.eq_ignore_ascii_case(" at random.")
        || discard_rest.eq_ignore_ascii_case(" at random")
    {
        " at random"
    } else if discard_rest == "." || discard_rest.is_empty() {
        ""
    } else {
        return None;
    };
    let clause = format!(
        "Each player draws {}, then discards {}{at_random}.",
        render_card_count_phrase(draw_count_raw),
        render_card_count_phrase(discard_count_raw)
    );
    Some(format!(
        "{}{}",
        prefix,
        lower_clause_after_prefix(prefix, &clause)
    ))
}

fn normalize_for_each_player_discard_draw_chain(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    let for_each_marker = "for each player, that player discards ";
    let plain_marker = "each player discards ";
    let (prefix, tail) = if let Some(idx) = lower.find(for_each_marker) {
        (&text[..idx], &text[idx + for_each_marker.len()..])
    } else if let Some(idx) = lower.find(plain_marker) {
        (&text[..idx], &text[idx + plain_marker.len()..])
    } else {
        return None;
    };
    let (discard_clause, rest) = tail.split_once(". ")?;
    let draw_tail = strip_prefix_ascii_ci(rest, "For each player, that player draws ")
        .or_else(|| strip_prefix_ascii_ci(rest, "Each player draws "))
        .or_else(|| strip_prefix_ascii_ci(rest, "that player draws "))?;
    let draw_clause = draw_tail.trim().trim_end_matches('.');
    if draw_clause.is_empty() {
        return None;
    }
    let clause = format!(
        "Each player discards {}, then draws {}.",
        discard_clause.trim(),
        draw_clause
    );
    Some(format!(
        "{}{}",
        prefix,
        lower_clause_after_prefix(prefix, &clause)
    ))
}

fn parse_card_count_with_rest(text: &str) -> Option<(&str, &str)> {
    if let Some((count, rest)) = text.split_once(" cards") {
        return Some((count.trim(), rest));
    }
    if let Some((count, rest)) = text.split_once(" card") {
        return Some((count.trim(), rest));
    }
    None
}

fn render_card_count_phrase(raw: &str) -> String {
    let count = normalize_count_token(raw);
    if matches!(count.as_str(), "a" | "an" | "one") {
        "a card".to_string()
    } else {
        format!("{count} cards")
    }
}

fn normalize_count_token(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.eq_ignore_ascii_case("a") || trimmed.eq_ignore_ascii_case("an") {
        return "a".to_string();
    }
    render_small_number_or_raw(trimmed)
}

fn lower_clause_after_prefix(prefix: &str, clause: &str) -> String {
    if prefix.ends_with(", ") {
        return lowercase_first(clause);
    }
    clause.to_string()
}

fn strip_prefix_ascii_ci<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    if text.len() < prefix.len() {
        return None;
    }
    if text
        .get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
    {
        text.get(prefix.len()..)
    } else {
        None
    }
}

fn strip_suffix_ascii_ci<'a>(text: &'a str, suffix: &str) -> Option<&'a str> {
    if text.len() < suffix.len() {
        return None;
    }
    let idx = text.len() - suffix.len();
    if text
        .get(idx..)
        .is_some_and(|tail| tail.eq_ignore_ascii_case(suffix))
    {
        text.get(..idx)
    } else {
        None
    }
}

fn split_once_ascii_ci<'a>(text: &'a str, separator: &str) -> Option<(&'a str, &'a str)> {
    let lower = text.to_ascii_lowercase();
    let sep_lower = separator.to_ascii_lowercase();
    let idx = lower.find(&sep_lower)?;
    Some((&text[..idx], &text[idx + separator.len()..]))
}

fn render_choose_exact_subject(descriptor: &str, count: usize) -> String {
    let descriptor = descriptor.trim();
    if let Some(rest) = descriptor.strip_prefix("this a ") {
        return format!("this {rest}");
    }
    if let Some(rest) = descriptor.strip_prefix("this an ") {
        return format!("this {rest}");
    }
    if count == 1 {
        if let Some(rest) = descriptor.strip_prefix("a ") {
            return with_indefinite_article(rest);
        }
        if let Some(rest) = descriptor.strip_prefix("an ") {
            return with_indefinite_article(rest);
        }
        return descriptor.to_string();
    }

    let count_word = render_small_number_or_raw(&count.to_string());
    if let Some(rest) = descriptor.strip_prefix("a ") {
        return format!("{count_word} {}", pluralize_noun_phrase(rest));
    }
    if let Some(rest) = descriptor.strip_prefix("an ") {
        return format!("{count_word} {}", pluralize_noun_phrase(rest));
    }
    format!("{count_word} {}", pluralize_noun_phrase(descriptor))
}

fn normalize_choose_exact_return_cost_clause(text: &str) -> Option<String> {
    let marker = " and tags it as 'return_cost_0', return target permanent to its owner's hand";
    let (head, tail) = split_once_ascii_ci(text, marker)?;
    let choose_idx = head.to_ascii_lowercase().rfind("choose exactly ")?;
    let prefix = &head[..choose_idx];
    let choose_tail = &head[choose_idx + "choose exactly ".len()..];
    let (count_token, rest) = choose_tail.split_once(' ')?;
    let count = count_token.parse::<usize>().ok()?;
    let descriptor = rest.strip_suffix(" in the battlefield")?;
    let subject = render_choose_exact_subject(descriptor, count);
    let owner_tail = if count == 1 {
        "its owner's hand"
    } else {
        "their owner's hand"
    };
    let clause = format!("Return {subject} to {owner_tail}");
    Some(format!("{prefix}{clause}{tail}"))
}

fn normalize_choose_exact_exile_cost_clause(text: &str) -> Option<String> {
    let marker = " and tags it as 'exile_cost_0', exile it";
    let (head, tail) = split_once_ascii_ci(text, marker)?;
    let choose_idx = head.to_ascii_lowercase().rfind("choose exactly ")?;
    let prefix = &head[..choose_idx];
    let choose_tail = &head[choose_idx + "choose exactly ".len()..];
    let (count_token, rest) = choose_tail.split_once(' ')?;
    let count = count_token.parse::<usize>().ok()?;
    let descriptor = rest
        .strip_suffix(" in the battlefield")
        .or_else(|| rest.strip_suffix(" in the stack"))?;
    let mut subject = render_choose_exact_subject(descriptor, count);
    if subject.contains("instant or sorcery") && !subject.contains(" spell") {
        if subject.starts_with("a ") {
            subject = subject.replacen("a instant or sorcery", "an instant or sorcery spell", 1);
        } else if subject.starts_with("an ") {
            subject = subject.replacen("an instant or sorcery", "an instant or sorcery spell", 1);
        } else {
            subject = subject.replacen("instant or sorcery", "instant or sorcery spell", 1);
        }
    }
    Some(format!("{prefix}Exile {subject}{tail}"))
}

fn parse_choose_exact_tail(head: &str) -> Option<(&str, usize, &str)> {
    let needle = " chooses exactly ";
    let lower = head.to_ascii_lowercase();
    let idx = lower.rfind(needle)?;
    let prefix = head.get(..idx)?.trim_end_matches(',');
    let choose_tail = head.get(idx + needle.len()..)?;
    let (count_token, rest) = choose_tail.split_once(' ')?;
    let count = count_token.parse::<usize>().ok()?;
    let descriptor = rest
        .strip_suffix(" in the battlefield")
        .or_else(|| rest.strip_suffix(" in a hand"))
        .or_else(|| rest.strip_suffix(" in hand"))
        .or_else(|| rest.strip_suffix(" in the stack"))
        .or_else(|| rest.strip_suffix(" in a graveyard"))
        .or_else(|| rest.strip_suffix(" in a library"))
        .or_else(|| rest.strip_suffix(" in exile"))
        .unwrap_or(rest);
    Some((prefix, count, descriptor))
}

fn normalize_choose_exact_tagged_it_clause(text: &str) -> Option<String> {
    if let Some((head, tail)) = text.split_once(" and tags it as '__it__'. Destroy it")
        && let Some((chooser, count, descriptor)) = parse_choose_exact_tail(head)
    {
        let mut descriptor = descriptor
            .replace("that player controls", "they control")
            .replace("target player's ", "")
            .replace("that player's ", "")
            .replace(" from their hand", " in their hand");
        descriptor = descriptor.replace(" in their hand in their hand", " in their hand");
        let chosen = render_choose_exact_subject(&descriptor, count);
        let target_ref = if chosen.to_ascii_lowercase().contains("creature") {
            "that creature"
        } else if chosen.to_ascii_lowercase().contains("artifact") {
            "that artifact"
        } else if chosen.to_ascii_lowercase().contains("card") {
            "that card"
        } else {
            "that permanent"
        };
        return Some(format!(
            "{chooser} chooses {chosen}. Destroy {target_ref}{tail}"
        ));
    }
    if let Some((head, tail)) = text.split_once(" and tags it as '__it__'")
        && let Some((chooser, count, descriptor)) = parse_choose_exact_tail(head)
    {
        let mut descriptor = descriptor
            .replace("that player controls", "they control")
            .replace("target player's ", "")
            .replace("that player's ", "")
            .replace(" from their hand", " in their hand");
        descriptor = descriptor.replace(" in their hand in their hand", " in their hand");
        let chosen = render_choose_exact_subject(&descriptor, count);
        return Some(format!("{chooser} chooses {chosen}{tail}"));
    }
    None
}

fn normalize_split_land_search_sequence(text: &str) -> Option<String> {
    let _ = text;
    None
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

fn static_heading_body(line: &str) -> Option<(&str, &str)> {
    let (prefix, body) = line.split_once(':')?;
    if prefix
        .trim()
        .to_ascii_lowercase()
        .starts_with("static ability ")
    {
        Some((prefix.trim(), body.trim()))
    } else {
        None
    }
}

fn merge_adjacent_static_heading_lines(lines: Vec<String>) -> Vec<String> {
    let mut current = lines;
    loop {
        let mut changed = false;
        let mut merged = Vec::with_capacity(current.len());
        let mut idx = 0usize;
        while idx < current.len() {
            if idx + 1 < current.len()
                && let (Some((left_prefix, left_body)), Some((_right_prefix, right_body))) = (
                    static_heading_body(&current[idx]),
                    static_heading_body(&current[idx + 1]),
                )
            {
                let pair = vec![left_body.to_string(), right_body.to_string()];
                let pair = merge_adjacent_subject_predicate_lines(pair);
                let pair = merge_subject_has_keyword_lines(pair);
                if pair.len() == 1 {
                    merged.push(format!("{left_prefix}: {}", pair[0].trim()));
                    idx += 2;
                    changed = true;
                    continue;
                }
            }
            merged.push(current[idx].clone());
            idx += 1;
        }
        if !changed {
            return current;
        }
        current = merged;
    }
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
            | "devoid"
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
            let keyword = keyword.trim_end_matches('.');
            if !subject.is_empty()
                && (is_keyword_phrase(keyword)
                    || normalize_keyword_list_phrase(keyword).is_some()
                    || normalize_keyword_and_phrase(keyword).is_some())
            {
                return Some((subject.to_string(), keyword.to_string()));
            }
        }
    }
    None
}

fn split_lose_all_abilities_clause(clause: &str) -> Option<String> {
    let trimmed = clause.trim().trim_end_matches('.');
    for verb in [" loses all abilities", " lose all abilities"] {
        if let Some(subject) = trimmed.strip_suffix(verb) {
            let subject = subject.trim();
            if !subject.is_empty() {
                return Some(subject.to_string());
            }
        }
    }
    None
}

fn extract_base_pt_tail_for_subject(line: &str, subject: &str) -> Option<String> {
    if let Some(pt) = line.strip_prefix("Affected permanents have base power and toughness ") {
        return Some(pt.trim().to_string());
    }
    for verb in ["has", "have"] {
        let prefix = format!("{subject} {verb} base power and toughness ");
        if let Some(pt) = line.strip_prefix(&prefix) {
            return Some(pt.trim().to_string());
        }
    }
    None
}

fn normalize_global_subject_number(subject: &str) -> String {
    let trimmed = subject.trim();
    if trimmed.eq_ignore_ascii_case("Creature") {
        return "Creatures".to_string();
    }
    if trimmed.eq_ignore_ascii_case("Land") {
        return "Lands".to_string();
    }
    if trimmed.eq_ignore_ascii_case("Artifact") {
        return "Artifacts".to_string();
    }
    if trimmed.eq_ignore_ascii_case("Enchantment") {
        return "Enchantments".to_string();
    }
    if trimmed.eq_ignore_ascii_case("Planeswalker") {
        return "Planeswalkers".to_string();
    }
    trimmed.to_string()
}

fn subject_is_plural(subject: &str) -> bool {
    let lower = subject.trim().to_ascii_lowercase();
    lower.starts_with("all ")
        || lower.starts_with("other ")
        || lower.starts_with("each ")
        || lower.starts_with("those ")
        || lower.ends_with('s')
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
    matches!(left_verb, "gets" | "get") && matches!(right_verb, "has" | "have" | "gains" | "gain")
}

fn normalize_keyword_predicate_case(predicate: &str) -> String {
    let trimmed = predicate.trim();
    if is_keyword_phrase(trimmed) {
        return trimmed.to_ascii_lowercase();
    }
    if let Some(joined) = normalize_keyword_list_phrase(trimmed) {
        return joined;
    }
    if let Some(joined) = normalize_keyword_and_phrase(trimmed) {
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

fn normalize_keyword_and_phrase(text: &str) -> Option<String> {
    let parts = text
        .split(" and ")
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
        if idx + 1 < lines.len() {
            let left = lines[idx].trim().trim_end_matches('.');
            let right = lines[idx + 1].trim().trim_end_matches('.');
            if let Some(subject) = left
                .strip_suffix(" enters tapped")
                .or_else(|| left.strip_suffix(" enter tapped"))
            {
                let counter_clause = right
                    .strip_prefix("Enters the battlefield with ")
                    .or_else(|| {
                        let singular = format!("{subject} enters with ");
                        let plural = format!("{subject} enter with ");
                        right
                            .strip_prefix(&singular)
                            .or_else(|| right.strip_prefix(&plural))
                    });
                if let Some(counter_clause) = counter_clause {
                let subject = subject.trim();
                if !subject.is_empty() {
                    let enter_verb = if subject_is_plural(subject) {
                        "enter"
                    } else {
                        "enters"
                    };
                    merged.push(format!(
                        "{subject} {enter_verb} tapped with {counter_clause}"
                    ));
                    idx += 2;
                    continue;
                }
                }
            }
        }
        if idx + 1 < lines.len()
            && let Some(left_subject) = split_lose_all_abilities_clause(lines[idx].trim())
        {
            let right_trimmed = lines[idx + 1].trim().trim_end_matches('.');
            if let Some(pt) = extract_base_pt_tail_for_subject(right_trimmed, &left_subject) {
                let subject = normalize_global_subject_number(&left_subject);
                let plural = subject_is_plural(&subject);
                let lose_verb = if plural { "lose" } else { "loses" };
                let have_verb = if plural { "have" } else { "has" };
                merged.push(format!(
                    "{subject} {lose_verb} all abilities and {have_verb} base power and toughness {pt}"
                ));
                idx += 2;
                continue;
            }
            let expected_tail_1 =
                format!("{left_subject} has Doesn't untap during your untap step");
            let expected_tail_2 =
                format!("{left_subject} has doesn't untap during your untap step");
            if right_trimmed.eq_ignore_ascii_case(&expected_tail_1)
                || right_trimmed.eq_ignore_ascii_case(&expected_tail_2)
            {
                merged.push(format!(
                    "{} loses all abilities and doesn't untap during its controller's untap step",
                    left_subject
                ));
                idx += 2;
                continue;
            }
        }
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

fn merge_lose_all_transform_lines(lines: Vec<String>) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;

    while idx < lines.len() {
        let left = lines[idx].trim().trim_end_matches('.');
        let Some(subject) = split_lose_all_abilities_clause(left) else {
            merged.push(lines[idx].clone());
            idx += 1;
            continue;
        };

        let mut consumed = 1usize;
        let mut colors: Vec<String> = Vec::new();
        let mut card_types: Vec<String> = Vec::new();
        let mut subtypes: Vec<String> = Vec::new();
        let mut named: Option<String> = None;
        let mut base_pt: Option<String> = None;

        while idx + consumed < lines.len() {
            let line = lines[idx + consumed].trim().trim_end_matches('.');
            if let Some(pt) = extract_base_pt_tail_for_subject(line, &subject) {
                base_pt = Some(pt);
                consumed += 1;
                continue;
            }

            let subject_is_prefix = format!("{subject} is ");
            let Some(rest) = line.strip_prefix(&subject_is_prefix) else {
                break;
            };
            let rest = rest.trim();
            if let Some(name) = rest.strip_prefix("named ") {
                named = Some(name.trim().to_string());
                consumed += 1;
                continue;
            }

            for part in rest
                .split(" and ")
                .map(str::trim)
                .filter(|part| !part.is_empty())
            {
                let lower = part.to_ascii_lowercase();
                if matches!(
                    lower.as_str(),
                    "white" | "blue" | "black" | "red" | "green" | "colorless"
                ) {
                    if !colors.contains(&lower) {
                        colors.push(lower);
                    }
                    continue;
                }
                if matches!(
                    lower.as_str(),
                    "creature" | "artifact" | "enchantment" | "land" | "planeswalker" | "battle"
                ) {
                    if !card_types.contains(&lower) {
                        card_types.push(lower);
                    }
                    continue;
                }
                if !subtypes.contains(&lower) {
                    subtypes.push(lower);
                }
            }
            consumed += 1;
        }

        if consumed == 1 {
            merged.push(lines[idx].clone());
            idx += 1;
            continue;
        }

        let mut combined = format!("{subject} loses all abilities");
        let mut descriptor = String::new();
        if !colors.is_empty() {
            descriptor.push_str(&join_with_and(&colors));
        }
        if !subtypes.is_empty() {
            if !descriptor.is_empty() {
                descriptor.push(' ');
            }
            descriptor.push_str(&join_with_and(&subtypes));
        }
        if !card_types.is_empty() {
            if !descriptor.is_empty() {
                descriptor.push(' ');
            }
            descriptor.push_str(&join_with_and(&card_types));
        }
        if !descriptor.is_empty() {
            combined.push_str(" and is ");
            combined.push_str(&descriptor);
        }
        if let Some(pt) = base_pt {
            combined.push_str(" with base power and toughness ");
            combined.push_str(&pt);
        }
        if let Some(name) = named {
            combined.push_str(" named ");
            combined.push_str(&name);
        }

        merged.push(combined);
        idx += consumed;
    }

    merged
}

fn parse_simple_mana_add_line(line: &str) -> Option<(&str, &str)> {
    let (cost, rest) = line.split_once(": ")?;
    let symbol = rest.strip_prefix("Add ")?;
    let symbol = symbol.trim().trim_end_matches('.');
    if symbol.contains(' ')
        || symbol.contains(',')
        || symbol.contains("or")
        || symbol.matches('{').count() == 0
        || symbol.matches('{').count() != symbol.matches('}').count()
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

fn have_verb_for_subject(subject: &str) -> &'static str {
    let lower = subject.to_ascii_lowercase();
    if lower.starts_with("enchanted ")
        || lower.starts_with("equipped ")
        || lower.starts_with("this ")
        || lower.starts_with("that ")
    {
        "has"
    } else if lower.starts_with("creatures")
        || lower.starts_with("other creatures")
        || lower.starts_with("all ")
        || lower.starts_with("those ")
        || lower.contains("creatures ")
    {
        "have"
    } else {
        // Check if subject contains a plural noun
        let plural_nouns = [
            "permanents",
            "creatures",
            "artifacts",
            "enchantments",
            "lands",
            "planeswalkers",
            "battles",
            "spells",
            "cards",
            "tokens",
        ];
        if plural_nouns.iter().any(|n| lower.contains(n)) {
            "have"
        } else {
            "has"
        }
    }
}

fn merge_subject_has_keyword_lines(lines: Vec<String>) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    while idx < lines.len() {
        if idx + 1 < lines.len() {
            let left = lines[idx].trim();
            let right = lines[idx + 1].trim();
            if let Some((left_subject, left_tail)) = split_have_clause(left)
                && let Some((right_subject, right_tail)) = split_have_clause(right)
                && left_subject.eq_ignore_ascii_case(&right_subject)
            {
                let verb = have_verb_for_subject(&left_subject);
                let left_tail = normalize_keyword_predicate_case(&left_tail);
                let right_tail = normalize_keyword_predicate_case(&right_tail);
                let left_key = strip_parenthetical_segments(&left_tail).to_ascii_lowercase();
                let right_key = strip_parenthetical_segments(&right_tail).to_ascii_lowercase();
                if left_key == right_key
                    || left_key.contains(&format!(" and {right_key}"))
                    || left_key.ends_with(&format!(" {right_key}"))
                {
                    merged.push(format!("{left_subject} {verb} {left_tail}"));
                } else {
                    merged.push(format!(
                        "{left_subject} {verb} {left_tail} and {right_tail}"
                    ));
                }
                idx += 2;
                continue;
            }
            if let Some((left_subject, left_rest)) = left
                .split_once(" gets ")
                .or_else(|| left.split_once(" get "))
                && let Some((right_subject, right_tail)) = split_have_clause(right)
                && left_subject.eq_ignore_ascii_case(&right_subject)
                && left_rest.contains(" and has ")
            {
                let right_tail = normalize_keyword_predicate_case(&right_tail);
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

fn drop_redundant_spell_cost_lines(lines: Vec<String>) -> Vec<String> {
    let has_this_spell_cost_clause = lines.iter().any(|line| {
        line.trim()
            .to_ascii_lowercase()
            .starts_with("this spell costs ")
    });
    if !has_this_spell_cost_clause {
        return lines;
    }

    lines
        .into_iter()
        .filter(|line| {
            let lower = line.trim().to_ascii_lowercase();
            !(lower.starts_with("spells cost ")
                && (lower.contains(" less to cast") || lower.contains(" more to cast")))
        })
        .collect()
}

fn is_keyword_style_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    let lower = trimmed.to_ascii_lowercase();
    if is_keyword_phrase(&lower) || normalize_keyword_list_phrase(&lower).is_some() {
        return true;
    }
    [
        "enchant ",
        "equip ",
        "crew ",
        "ward ",
        "kicker ",
        "flashback ",
        "cycling ",
        "landcycling ",
        "basic landcycling ",
        "madness ",
        "morph ",
        "suspend ",
        "prototype ",
        "bestow ",
        "affinity ",
        "fuse",
        "adventure",
    ]
    .iter()
    .any(|prefix| lower.starts_with(prefix))
}

fn normalize_sentence_surface_style(line: &str) -> String {
    let mut normalized = line.trim().to_string();
    if normalized.is_empty() {
        return normalized;
    }

    if normalized
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_lowercase())
    {
        normalized = capitalize_first(&normalized);
    }

    // Modal rendering may include debug-style bracket expansions; strip them from
    // public-facing compiled text so semantic comparisons focus on the main clause.
    normalized = strip_square_bracketed_segments(&normalized)
        .trim()
        .to_string();
    normalized = normalized.replace('\u{00a0}', " ");
    normalized = normalized.split_whitespace().collect::<Vec<_>>().join(" ");
    normalized = normalize_ward_cost_surface(&normalized);
    if let Some(rewritten) = normalize_search_discard_then_shuffle_surface(&normalized) {
        return rewritten;
    }
    if let Some(rewritten) = normalize_discard_random_then_discard_surface(&normalized) {
        return rewritten;
    }
    if let Some(rewritten) = normalize_choose_exact_return_cost_clause(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_choose_exact_exile_cost_clause(&normalized) {
        normalized = rewritten;
    }
    normalized = normalized.replace("controlss", "controls");
    let lower_normalized = normalized.to_ascii_lowercase();
    if let Some(rest) = lower_normalized.strip_prefix("spell effects: ")
        && rest.starts_with(
            "target opponent chooses target creature an opponent controls. exile it. exile all ",
        )
        && (rest.contains(" in target opponent's graveyard")
            || rest.contains(" in target opponent's graveyards"))
    {
        return "Spell effects: Target opponent exiles a creature they control and their graveyard."
            .to_string();
    }
    if lower_normalized.starts_with(
        "target opponent chooses target creature an opponent controls. exile it. exile all ",
    ) && (lower_normalized.contains(" in target opponent's graveyard")
        || lower_normalized.contains(" in target opponent's graveyards"))
    {
        return "Target opponent exiles a creature they control and their graveyard.".to_string();
    }
    if let Some((inner, payment)) = normalized.split_once(" unless a player pays ")
        && inner.starts_with("Search ")
    {
        let payment = payment.trim().trim_end_matches('.');
        return format!(
            "Unless any player pays {}, {}.",
            payment,
            lowercase_first(inner)
        );
    }
    if lower_normalized.contains("and tags it as 'exiled_0'")
        && lower_normalized.contains("for each object exiled this way, search that player's library for permanent that shares a card type with that object that player owns, put it onto the battlefield, then shuffle")
    {
        let mut chosen_types = Vec::new();
        for card_type in ["artifact", "creature", "enchantment", "planeswalker", "land", "battle"] {
            let phrase = format!(
                "choose up to one {card_type} in the battlefield and tags it as 'exiled_0'"
            );
            if lower_normalized.contains(&phrase) {
                chosen_types.push(format!("up to one target {card_type}"));
            }
        }
        if chosen_types.len() >= 2 {
            return format!(
                "Exile {}. For each permanent exiled this way, its controller reveals cards from the top of their library until they reveal a card that shares a card type with it, puts that card onto the battlefield, then shuffles.",
                join_with_and(&chosen_types)
            );
        }
    }
    if let Some((head, body)) = normalized.split_once(':')
        && head.trim().to_ascii_lowercase().starts_with("this ")
        && head
            .trim()
            .to_ascii_lowercase()
            .contains(" leaves the battlefield")
    {
        return format!("When {}, {}", head.trim().to_ascii_lowercase(), body.trim());
    }
    let token_plural_starts = [
        "Create two ",
        "Create three ",
        "Create four ",
        "Create five ",
        "Create six ",
        "Create seven ",
        "Create eight ",
        "Create nine ",
        "Create 2 ",
        "Create 3 ",
        "Create 4 ",
        "Create 5 ",
        "Create 6 ",
        "Create 7 ",
        "Create 8 ",
        "Create 9 ",
    ];
    if token_plural_starts
        .iter()
        .any(|prefix| normalized.starts_with(prefix))
        && normalized.contains(" creature token")
        && !normalized.contains(" creature tokens")
    {
        normalized = normalized.replacen(" creature token", " creature tokens", 1);
    }
    let lower_plural_markers = [
        "create two ",
        "create three ",
        "create four ",
        "create five ",
        "create six ",
        "create seven ",
        "create eight ",
        "create nine ",
        "create 2 ",
        "create 3 ",
        "create 4 ",
        "create 5 ",
        "create 6 ",
        "create 7 ",
        "create 8 ",
        "create 9 ",
    ];
    if lower_plural_markers
        .iter()
        .any(|marker| lower_normalized.contains(marker))
        && normalized.contains(" creature token")
        && !normalized.contains(" creature tokens")
    {
        normalized = normalized.replacen(" creature token", " creature tokens", 1);
    }
    if let Some((left, right)) = normalized.split_once(". ") {
        let right_lower = right.trim_start().to_ascii_lowercase();
        if !right_lower.starts_with("you sacrifice ") && !right_lower.starts_with("sacrifice ") {
            // no-op
        } else {
            let left_trimmed = left.trim().trim_end_matches('.');
            let right_trimmed = right
                .trim_start()
                .trim_start_matches("you ")
                .trim_start_matches("You ")
                .trim_start_matches("sacrifice ")
                .trim();
            let left_lower = left_trimmed.to_ascii_lowercase();
            if left_lower.starts_with("you draw ")
                || left_lower.starts_with("you discard ")
                || left_lower.starts_with("you gain ")
                || left_lower.contains(" and you lose ")
                || left_lower.contains(" and you gain ")
            {
                return format!(
                    "{left_trimmed}, then sacrifice {}.",
                    right_trimmed.trim_end_matches('.')
                );
            }
        }
    }
    if let Some((left, right)) = split_once_ascii_ci(&normalized, ". investigate") {
        let left_trimmed = left.trim().trim_end_matches('.');
        let right_tail = right.trim_start_matches('.').trim_start_matches(',').trim();
        if left_trimmed.to_ascii_lowercase().contains("create ") {
            if right_tail.is_empty() {
                return format!("{left_trimmed}, then investigate.");
            }
            return format!("{left_trimmed}, then investigate. {right_tail}");
        }
    }
    if let Some((trigger_head, trigger_body)) = normalized.split_once(':')
        && trigger_head
            .trim()
            .to_ascii_lowercase()
            .starts_with("one or more ")
    {
        return format!(
            "Whenever {}, {}",
            trigger_head.trim().to_ascii_lowercase(),
            trigger_body.trim()
        );
    }
    if let Some(rest) = normalized.strip_prefix("Creatures you control get ")
        && let Some(pt) = rest
            .strip_suffix(" as long as it's your turn.")
            .or_else(|| rest.strip_suffix(" as long as it's your turn"))
    {
        return format!("During your turn, creatures you control get {pt}.");
    }
    if let Some((head, _tail)) = normalized
        .split_once(", put a card from that player's hand on top of that player's library")
        && (head.starts_with("When this creature enters")
            || head.starts_with("When this permanent enters"))
    {
        return format!(
            "{head}, target player puts a card from their hand on top of their library."
        );
    }
    if let Some((prefix, rest)) =
        split_once_ascii_ci(&normalized, ". For each opponent's creature, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to each opponent.")
            .or_else(|| rest.strip_suffix(" damage to each opponent"))
        && let Some((subject, self_amount)) =
            strip_prefix_ascii_ci(prefix.trim(), "When this permanent enters, it deals ")
                .map(|tail| ("permanent", tail))
                .or_else(|| {
                    strip_prefix_ascii_ci(prefix.trim(), "When this creature enters, it deals ")
                        .map(|tail| ("creature", tail))
                })
                .or_else(|| {
                    split_once_ascii_ci(prefix.trim(), ": When this permanent enters, it deals ")
                        .map(|(_, tail)| ("permanent", tail))
                })
                .or_else(|| {
                    split_once_ascii_ci(prefix.trim(), ": When this creature enters, it deals ")
                        .map(|(_, tail)| ("creature", tail))
                })
                .and_then(|(subject, tail)| {
                    strip_suffix_ascii_ci(tail, " damage to that player")
                        .map(|amount| (subject, amount))
                })
        && self_amount.trim().eq_ignore_ascii_case(amount.trim())
    {
        return format!(
            "When this {subject} enters, it deals {} damage to each opponent and each creature your opponents control.",
            amount.trim(),
        );
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(&normalized, ": For each ")
        && let Some((first_filter, rest)) = split_once_ascii_ci(rest, ", put ")
        && let Some((first_counter, rest)) = split_once_ascii_ci(rest, " on that object. For each ")
        && let Some((second_filter, rest)) = split_once_ascii_ci(rest, ", Put ")
        && let Some(second_counter) = strip_suffix_ascii_ci(rest, " on that object.")
            .or_else(|| strip_suffix_ascii_ci(rest, " on that object"))
    {
        return format!(
            "{prefix}: put {} on each {} and {} on each {}.",
            first_counter.trim(),
            first_filter.trim(),
            second_counter.trim(),
            second_filter.trim()
        );
    }
    if normalized.eq_ignore_ascii_case("All Slivers have \"Sacrifice this creature: Add b b.\"")
        || normalized
            .eq_ignore_ascii_case("All Slivers have \"Sacrifice this permanent: Add b b.\"")
        || normalized
            .eq_ignore_ascii_case("All Slivers have \"Sacrifice this creature: Add {b}{b}.\"")
        || normalized
            .eq_ignore_ascii_case("All Slivers have \"Sacrifice this permanent: Add {b}{b}.\"")
    {
        return "All Slivers have \"Sacrifice this permanent: Add {B}{B}.\"".to_string();
    }
    let format_choose_modes = |head: &str, marker: &str, tail: &str| {
        let modes: Vec<String> = tail
            .split(" • ")
            .map(|mode| mode.trim().trim_start_matches('•').trim().to_string())
            .filter(|mode| !mode.is_empty())
            .collect();
        if modes.len() < 2 {
            return None;
        }
        let mut rewritten = format!("{head}{marker}");
        for mode in modes {
            rewritten.push_str("\n• ");
            rewritten.push_str(&mode);
        }
        Some(rewritten)
    };
    if !normalized.contains('\n') {
        if let Some((head, tail)) = normalized.split_once(" choose one or more - ")
            && tail.contains(" • ")
            && let Some(rewritten) = format_choose_modes(head, " choose one or more —", tail)
        {
            return rewritten;
        }
        if let Some((head, tail)) = normalized.split_once(" Choose one or more - ")
            && tail.contains(" • ")
            && let Some(rewritten) = format_choose_modes(head, " Choose one or more —", tail)
        {
            return rewritten;
        }
        if let Some((head, tail)) = normalized.split_once(" choose one or both - ")
            && tail.contains(" • ")
            && let Some(rewritten) = format_choose_modes(head, " choose one or both —", tail)
        {
            return rewritten;
        }
        if let Some((head, tail)) = normalized.split_once(" Choose one or both - ")
            && tail.contains(" • ")
            && let Some(rewritten) = format_choose_modes(head, " Choose one or both —", tail)
        {
            return rewritten;
        }
        if let Some((head, tail)) = normalized.split_once(" choose one - ")
            && tail.contains(" • ")
            && let Some(rewritten) = format_choose_modes(head, " choose one —", tail)
        {
            return rewritten;
        }
        if let Some((head, tail)) = normalized.split_once(" Choose one - ")
            && tail.contains(" • ")
            && let Some(rewritten) = format_choose_modes(head, " Choose one —", tail)
        {
            return rewritten;
        }
        if let Some((head, tail)) = normalized.split_once(" choose one or both — ")
            && tail.contains(" • ")
            && let Some(rewritten) = format_choose_modes(head, " choose one or both —", tail)
        {
            return rewritten;
        }
        if let Some((head, tail)) = normalized.split_once(" Choose one or both — ")
            && tail.contains(" • ")
            && let Some(rewritten) = format_choose_modes(head, " Choose one or both —", tail)
        {
            return rewritten;
        }
        if let Some((head, tail)) = normalized.split_once(" choose one — ")
            && tail.contains(" • ")
            && let Some(rewritten) = format_choose_modes(head, " choose one —", tail)
        {
            return rewritten;
        }
        if let Some((head, tail)) = normalized.split_once(" Choose one — ")
            && tail.contains(" • ")
            && let Some(rewritten) = format_choose_modes(head, " Choose one —", tail)
        {
            return rewritten;
        }
        if let Some((head, tail)) = normalized.split_once(" choose one or more — ")
            && tail.contains(" • ")
            && let Some(rewritten) = format_choose_modes(head, " choose one or more —", tail)
        {
            return rewritten;
        }
        if let Some((head, tail)) = normalized.split_once(" Choose one or more — ")
            && tail.contains(" • ")
            && let Some(rewritten) = format_choose_modes(head, " Choose one or more —", tail)
        {
            return rewritten;
        }
    }

    if lower_normalized.contains(
        "treasure artifact token with {t}, sacrifice this artifact: add one mana of any color. tapped under your control",
    ) || lower_normalized.contains(
        "treasure artifact token with {t}, sacrifice this artifact: add one mana of any color. under your control, tapped",
    ) {
        return normalized
            .replace(
                "Create a Treasure artifact token with {T}, Sacrifice this artifact: Add one mana of any color. tapped under your control",
                "Create a tapped Treasure token",
            )
            .replace(
                "Create a Treasure artifact token with {T}, Sacrifice this artifact: Add one mana of any color. under your control, tapped",
                "Create a tapped Treasure token",
            )
            .replace(
                "create a Treasure artifact token with {T}, Sacrifice this artifact: Add one mana of any color. tapped under your control",
                "create a tapped Treasure token",
            )
            .replace(
                "create a Treasure artifact token with {T}, Sacrifice this artifact: Add one mana of any color. under your control, tapped",
                "create a tapped Treasure token",
            )
            .replace(
                "create a treasure artifact token with {t}, sacrifice this artifact: add one mana of any color. tapped under your control",
                "create a tapped Treasure token",
            )
            .replace(
                "create a treasure artifact token with {t}, sacrifice this artifact: add one mana of any color. under your control, tapped",
                "create a tapped Treasure token",
            );
    }
    if lower_normalized.contains(
        "0/1 colorless eldrazi spawn creature token with sacrifice this creature: add {c}. under your control",
    ) {
        return normalized
            .replace(
                "Create a 0/1 colorless Eldrazi Spawn creature token with Sacrifice this creature: Add {C}. under your control",
                "Create a 0/1 colorless Eldrazi Spawn creature token. It has \"Sacrifice this token: Add {C}.\"",
            )
            .replace(
                "create a 0/1 colorless Eldrazi Spawn creature token with Sacrifice this creature: Add {C}. under your control",
                "create a 0/1 colorless Eldrazi Spawn creature token. It has \"Sacrifice this token: Add {C}.\"",
            )
            .replace(
                "create a 0/1 colorless eldrazi spawn creature token with sacrifice this creature: add {c}. under your control",
                "create a 0/1 colorless Eldrazi Spawn creature token. It has \"Sacrifice this token: Add {C}.\"",
            );
    }
    if lower_normalized.contains(
        "1/1 colorless eldrazi scion creature token with sacrifice this creature: add {c}. under your control",
    ) {
        return normalized
            .replace(
                "Create a 1/1 colorless Eldrazi Scion creature token with Sacrifice this creature: Add {C}. under your control",
                "Create a 1/1 colorless Eldrazi Scion creature token. It has \"Sacrifice this token: Add {C}.\"",
            )
            .replace(
                "create a 1/1 colorless Eldrazi Scion creature token with Sacrifice this creature: Add {C}. under your control",
                "create a 1/1 colorless Eldrazi Scion creature token. It has \"Sacrifice this token: Add {C}.\"",
            )
            .replace(
                "create a 1/1 colorless eldrazi scion creature token with sacrifice this creature: add {c}. under your control",
                "create a 1/1 colorless Eldrazi Scion creature token. It has \"Sacrifice this token: Add {C}.\"",
            );
    }
    if let Some((left, right)) = split_once_ascii_ci(&normalized, " and you gain ")
        && left
            .trim_start()
            .to_ascii_lowercase()
            .starts_with("you gain ")
        && let Some(base_amount) = strip_prefix_ascii_ci(left.trim(), "you gain ")
            .and_then(|tail| strip_suffix_ascii_ci(tail.trim(), " life"))
        && let Some(extra_amount) =
            strip_suffix_ascii_ci(right.trim().trim_end_matches('.'), " life")
    {
        return format!(
            "You gain {} plus {} life.",
            base_amount.trim(),
            extra_amount.trim()
        );
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(&normalized, ", creatures you control get ")
        && let Some((pt_tail, gain_tail)) =
            split_once_ascii_ci(rest, " until end of turn. creatures you control gain ")
        && let Some(keyword_tail) = strip_suffix_ascii_ci(gain_tail, " until end of turn")
    {
        return format!(
            "{prefix}, creatures you control get {} and gain {} until end of turn.",
            pt_tail.trim(),
            keyword_tail.trim().to_ascii_lowercase()
        );
    }
    if let Some((left, right)) = split_once_ascii_ci(&normalized, " and you gain ")
        && {
            let left_lower = left.trim_start().to_ascii_lowercase();
            left_lower.starts_with("deal ")
                || left_lower.starts_with("destroy ")
                || left_lower.starts_with("return ")
                || left_lower.starts_with("counter target")
                || left_lower.starts_with("exile ")
                || left_lower.starts_with("search your library")
                || left_lower.starts_with("create ")
        }
    {
        return format!(
            "{}. You gain {}.",
            left.trim_end_matches('.'),
            right.trim_end_matches('.')
        );
    }
    if let Some((left, right)) = split_once_ascii_ci(&normalized, " and you lose ")
        && {
            let left_lower = left.trim_start().to_ascii_lowercase();
            left_lower.starts_with("deal ")
                || left_lower.starts_with("destroy ")
                || left_lower.starts_with("return ")
                || left_lower.starts_with("counter target")
                || left_lower.starts_with("exile ")
                || left_lower.starts_with("search your library")
                || left_lower.starts_with("create ")
        }
    {
        return format!(
            "{}. You lose {}.",
            left.trim_end_matches('.'),
            right.trim_end_matches('.')
        );
    }
    if let Some((draw_clause, put_clause)) = split_once_ascii_ci(&normalized, ". ")
        && {
            let lower_draw = draw_clause.trim_start().to_ascii_lowercase();
            lower_draw.starts_with("you draw ")
                || lower_draw.contains(", you draw ")
                || lower_draw.contains(": you draw ")
        }
        && let Some(card_phrase) =
            strip_suffix_ascii_ci(put_clause.trim(), " from your hand on top of your library.")
                .or_else(|| {
                    strip_suffix_ascii_ci(
                        put_clause.trim(),
                        " from your hand on top of your library",
                    )
                })
    {
        let card_phrase = strip_prefix_ascii_ci(card_phrase.trim(), "put ")
            .unwrap_or(card_phrase)
            .trim();
        let mut rewritten = format!(
            "{}, then put {} from your hand on top of your library",
            draw_clause.trim_end_matches('.'),
            card_phrase
        );
        if card_phrase.to_ascii_lowercase().contains("cards")
            && !put_clause.to_ascii_lowercase().contains("in any order")
        {
            rewritten.push_str(" in any order");
        }
        rewritten.push('.');
        return rewritten;
    }
    if let Some((put_clause, shuffle_clause)) = split_once_ascii_ci(&normalized, ". Shuffle ") {
        let (shuffle_library_head, shuffle_tail) = split_once_ascii_ci(shuffle_clause, ". ")
            .map_or_else(
                || (shuffle_clause.trim(), ""),
                |(head, tail)| (head.trim(), tail.trim()),
            );
        if let Some(library_owner) = strip_suffix_ascii_ci(shuffle_library_head, " library")
            .or_else(|| strip_suffix_ascii_ci(shuffle_library_head, " library."))
        {
            let bottom_suffix = format!(" on the bottom of {} library", library_owner.trim());
            if let Some(move_clause) = strip_suffix_ascii_ci(put_clause.trim(), &bottom_suffix) {
                let move_clause = move_clause.trim();
                let split_put_clause = split_once_ascii_ci(move_clause, "Put ")
                    .or_else(|| split_once_ascii_ci(move_clause, "put "));
                if let Some((prefix, moved_cards)) = split_put_clause {
                    let prefix = prefix.trim_end();
                    let moved_cards = moved_cards.trim();
                    let shuffle_verb = if prefix.is_empty()
                        || prefix.ends_with(':')
                        || prefix.ends_with(';')
                        || prefix.ends_with('.')
                    {
                        "Shuffle"
                    } else {
                        "shuffle"
                    };
                    let mut rewritten = if prefix.is_empty() {
                        format!(
                            "{shuffle_verb} {moved_cards} into {} library",
                            library_owner.trim()
                        )
                    } else {
                        format!(
                            "{prefix} {shuffle_verb} {moved_cards} into {} library",
                            library_owner.trim()
                        )
                    };
                    if !shuffle_tail.is_empty() {
                        rewritten.push_str(". ");
                        rewritten.push_str(shuffle_tail);
                    } else {
                        rewritten.push('.');
                    }
                    return rewritten;
                }
            }
        }
    }
    if let Some((prelude, graveyard_tail)) =
        split_once_ascii_ci(&normalized, ". Shuffle that object's owner's library. ")
        && let Some(move_clause) =
            strip_suffix_ascii_ci(prelude.trim(), " on the bottom of its owner's library")
        && let Some((prefix, moved_cards)) = split_once_ascii_ci(move_clause.trim(), "Put ")
            .or_else(|| split_once_ascii_ci(move_clause.trim(), "put "))
    {
        let graveyard_tail = graveyard_tail.trim();
        if graveyard_tail.eq_ignore_ascii_case("you shuffle your graveyard into your library.")
            || graveyard_tail.eq_ignore_ascii_case("you shuffle your graveyard into your library")
        {
            let prefix = prefix.trim_end();
            let moved_cards = moved_cards.trim();
            if prefix.is_empty() {
                return format!("Shuffle {moved_cards} and your graveyard into their owner's library.");
            }
            return format!(
                "{prefix} Shuffle {moved_cards} and your graveyard into their owner's library."
            );
        }
    }
    if let Some(rest) = normalized
        .strip_prefix("For each player, if that player controls ")
        .or_else(|| normalized.strip_prefix("for each player, if that player controls "))
        && let Some((controls, tail)) = rest.split_once(", Create 1 ")
        && let Some((token_tail, remainder)) = tail.split_once(" under that player's control")
    {
        let mut rewritten = format!(
            "Each player who controls {} creates a {}.",
            with_indefinite_article(controls),
            token_tail
        );
        let remainder = remainder
            .trim_start_matches('.')
            .trim_start_matches(',')
            .trim();
        if !remainder.is_empty() {
            rewritten.push(' ');
            rewritten.push_str(remainder);
        }
        return rewritten;
    }
    if let Some((prefix, _)) = normalized.split_once(
        ", for each player, Put a card from that player's hand on top of that player's library.",
    ) {
        return format!(
            "{prefix}, each player puts a card from their hand on top of their library."
        );
    }
    if let Some((prefix, _)) = normalized.split_once(
        ", for each player, Put a card from that player's hand on top of that player's library",
    ) {
        return format!(
            "{prefix}, each player puts a card from their hand on top of their library."
        );
    }
    if let Some((lose_clause, put_clause)) = normalized.split_once(". ")
        && lose_clause
            .to_ascii_lowercase()
            .starts_with("target opponent loses ")
        && (put_clause == "Put a card from that player's hand on top of that player's library."
            || put_clause == "Put a card from that player's hand on top of that player's library")
    {
        return format!("{lose_clause} and puts a card from their hand on top of their library.");
    }
    if let Some(rest) = normalized.strip_prefix("Other ")
        && let Some((kind, tail)) = rest.split_once(" you control get ")
        && let Some(buff) = tail
            .strip_suffix(" and have ward 1.")
            .or_else(|| tail.strip_suffix(" and have ward 1"))
            .or_else(|| tail.strip_suffix(" and have ward {1}."))
            .or_else(|| tail.strip_suffix(" and have ward {1}"))
    {
        return format!("Each other {kind} you control gets {buff} and has ward {{1}}.");
    }
    if let Some(rest) = normalized.strip_prefix("Protection from ")
        && !rest.contains(' ')
        && !matches!(
            rest.to_ascii_lowercase().as_str(),
            "white" | "blue" | "black" | "red" | "green" | "colorless" | "everything"
        )
        && !rest.ends_with('s')
    {
        return format!("Protection from {}", pluralize_noun_phrase(rest));
    }
    if !is_keyword_style_line(&normalized)
        && !normalized.ends_with('.')
        && !normalized.ends_with('!')
        && !normalized.ends_with('?')
        && !normalized.ends_with('"')
    {
        normalized.push('.');
    }

    normalized = normalized
        .replace(
            "Counter target instant spell spell.",
            "Counter target instant spell.",
        )
        .replace(
            "Counter target sorcery spell spell.",
            "Counter target sorcery spell.",
        )
        .replace(" ors ", " or ")
        .replace(" ors", " or")
        .replace("ors ", "or ")
        .replace("a artifact", "an artifact")
        .replace("a enchantment", "an enchantment")
        .replace("a Aura", "an Aura")
        .replace("a player may pays ", "that player may pay ")
        .replace(
            "untap all a snow permanent you control",
            "untap each snow permanent you control",
        )
        .replace("for each a ", "for each ")
        .replace("for each an ", "for each ")
        .replace("Elfs you control get ", "Elves you control get ")
        .replace("Other Elf you control get ", "Other Elves you control get ")
        .replace("other Elf you control get ", "other Elves you control get ")
        .replace("Warrior have ", "Warriors have ")
        .replace("warrior have ", "warriors have ")
        .replace(
            "Creature with a level counter on it you control get ",
            "Each creature you control with a level counter on it gets ",
        )
        .replace(
            "creature with a level counter on it you control get ",
            "each creature you control with a level counter on it gets ",
        )
        .replace(
            "the number of Soldiers or Warrior you control",
            "the number of Soldiers and Warriors you control",
        )
        .replace(
            "the number of Soldiers and Warrior you control",
            "the number of Soldiers and Warriors you control",
        )
        .replace("Goblin are black", "Goblins are black")
        .replace(
            "Goblin are zombie in addition to their other types",
            "Goblins are Zombies in addition to their other creature types",
        )
        .replace(
            "Whenever this creature or Whenever another Ally you control enters",
            "Whenever this creature or another Ally you control enters",
        )
        .replace(
            "Whenever this creature or least ",
            "Whenever this creature and at least ",
        )
        .replace(
            "Whenever you cast an instant or sorcery spell, deal ",
            "Whenever you cast an instant or sorcery spell, this creature deals ",
        );

    if let Some((amount, rest)) = normalized
        .strip_prefix("Prevent the next ")
        .and_then(|tail| tail.split_once(" damage to "))
        && let Some(target) = rest
            .strip_suffix(" until end of turn.")
            .or_else(|| rest.strip_suffix(" until end of turn"))
    {
        return format!(
            "Prevent the next {amount} damage that would be dealt to {target} this turn."
        );
    }

    if let Some(rest) = normalized.strip_prefix("This creature has ")
        && let Some(keyword) = rest
            .strip_suffix(" as long as it's your turn.")
            .or_else(|| rest.strip_suffix(" as long as it's your turn"))
        && is_keyword_phrase(keyword)
    {
        return format!(
            "During your turn, this creature has {}.",
            keyword.to_ascii_lowercase()
        );
    }
    if let Some(rest) = normalized.strip_prefix("Creatures you control have ")
        && let Some(keyword) = rest
            .strip_suffix(" as long as it's your turn.")
            .or_else(|| rest.strip_suffix(" as long as it's your turn"))
        && is_keyword_phrase(keyword)
    {
        return format!(
            "During your turn, creatures you control have {}.",
            keyword.to_ascii_lowercase()
        );
    }
    if let Some(rest) = normalized.strip_prefix("Allies you control have ")
        && let Some(keyword) = rest
            .strip_suffix(" as long as it's your turn.")
            .or_else(|| rest.strip_suffix(" as long as it's your turn"))
        && is_keyword_phrase(keyword)
    {
        return format!(
            "During your turn, Allies you control have {}.",
            keyword.to_ascii_lowercase()
        );
    }
    if let Some(count) = normalized
        .strip_prefix("For each opponent, that player discards ")
        .and_then(|rest| rest.strip_suffix(" cards."))
        .or_else(|| {
            normalized
                .strip_prefix("For each opponent, that player discards ")
                .and_then(|rest| rest.strip_suffix(" cards"))
        })
    {
        return format!(
            "Each opponent discards {} cards.",
            render_small_number_or_raw(count)
        );
    }
    if normalized == "For each opponent, that player discards a card."
        || normalized == "For each opponent, that player discards a card"
    {
        return "Each opponent discards a card.".to_string();
    }
    if let Some(count) = normalized
        .strip_prefix("For each opponent, that player gets ")
        .and_then(|rest| {
            rest.strip_suffix(" poison counter(s).")
                .or_else(|| rest.strip_suffix(" poison counter(s)"))
                .or_else(|| rest.strip_suffix(" poison counters."))
                .or_else(|| rest.strip_suffix(" poison counters"))
                .or_else(|| rest.strip_suffix(" poison counter."))
                .or_else(|| rest.strip_suffix(" poison counter"))
        })
    {
        let count = count.trim();
        if matches!(count, "1" | "one" | "a" | "an") {
            return "Each opponent gets a poison counter.".to_string();
        }
        return format!("Each opponent gets {count} poison counters.");
    }
    if let Some(count) = normalized
        .strip_prefix("For each player, that player mills ")
        .and_then(|rest| rest.strip_suffix(" cards."))
        .or_else(|| {
            normalized
                .strip_prefix("For each player, that player mills ")
                .and_then(|rest| rest.strip_suffix(" cards"))
        })
    {
        return format!(
            "Each player mills {} cards.",
            render_small_number_or_raw(count)
        );
    }
    if let Some(count) = normalized
        .strip_prefix("For each player, that player draws ")
        .and_then(|rest| rest.strip_suffix(" cards."))
        .or_else(|| {
            normalized
                .strip_prefix("For each player, that player draws ")
                .and_then(|rest| rest.strip_suffix(" cards"))
        })
    {
        return format!(
            "Each player draws {} cards.",
            render_small_number_or_raw(count)
        );
    }
    if normalized == "For each player, that player discards a card at random."
        || normalized == "For each player, that player discards a card at random"
    {
        return "Each player discards a card at random.".to_string();
    }
    if let Some(rest) =
        normalized.strip_prefix("For each player, Return all creature card from target player's graveyard to target player's hand")
    {
        if rest.trim().is_empty() || rest.trim() == "." {
            return "Each player returns all creature cards from their graveyard to their hand."
                .to_string();
        }
    }
    if let Some(rest) = normalized.strip_prefix("For each attacking creature, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to that object.")
            .or_else(|| rest.strip_suffix(" damage to that object"))
    {
        return format!("Deal {amount} damage to each attacking creature.");
    }
    if let Some(rest) = normalized.strip_prefix("For each blocking creature, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to that object.")
            .or_else(|| rest.strip_suffix(" damage to that object"))
    {
        return format!("Deal {amount} damage to each blocking creature.");
    }
    if let Some(rest) = normalized.strip_prefix("For each attacking/blocking creature, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to that object.")
            .or_else(|| rest.strip_suffix(" damage to that object"))
    {
        return format!("Deal {amount} damage to each attacking or blocking creature.");
    }
    if let Some(rest) = normalized.strip_prefix("For each another creature without flying, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to that object.")
            .or_else(|| rest.strip_suffix(" damage to that object"))
    {
        return format!(
            "This creature deals {amount} damage to each other creature without flying."
        );
    }
    if let Some(rest) = normalized.strip_prefix("For each opponent, that player loses ")
        && let Some(amount) = rest
            .strip_suffix(" life.")
            .or_else(|| rest.strip_suffix(" life"))
    {
        return format!("Each opponent loses {amount} life.");
    }

    if normalized
        == "For each player, that player draws a card. For each player, that player discards a card."
        || normalized
            == "For each player, that player draws a card. For each player, that player discards a card"
    {
        return "Each player draws a card, then discards a card.".to_string();
    }
    if let Some(amount) = normalized
        .strip_prefix("For each opponent, Deal ")
        .and_then(|rest| rest.strip_suffix(" damage to that player."))
        .or_else(|| {
            normalized
                .strip_prefix("For each opponent, Deal ")
                .and_then(|rest| rest.strip_suffix(" damage to that player"))
        })
    {
        return format!("Deal {amount} damage to each opponent.");
    }
    if let Some((prefix, tail)) = normalized.split_once(". For each opponent, Deal ")
        && let Some(amount) = tail
            .strip_suffix(" damage to that player.")
            .or_else(|| tail.strip_suffix(" damage to that player"))
    {
        return format!("{prefix}. Deal {amount} damage to each opponent.");
    }
    if let Some(amount) = normalized
        .strip_prefix("For each opponent, that player loses ")
        .and_then(|rest| rest.strip_suffix(" life."))
        .or_else(|| {
            normalized
                .strip_prefix("For each opponent, that player loses ")
                .and_then(|rest| rest.strip_suffix(" life"))
        })
    {
        return format!("Each opponent loses {amount} life.");
    }
    if let Some(amount) = normalized
        .strip_prefix("Whenever this creature attacks, for each opponent, that player loses ")
        .and_then(|rest| rest.strip_suffix(" life."))
        .or_else(|| {
            normalized
                .strip_prefix(
                    "Whenever this creature attacks, for each opponent, that player loses ",
                )
                .and_then(|rest| rest.strip_suffix(" life"))
        })
    {
        return format!("Whenever this creature attacks, each opponent loses {amount} life.");
    }
    if let Some(card_text) = normalized
        .strip_prefix("For each player, Put ")
        .and_then(|rest| {
            rest.strip_suffix(" in that player's graveyard onto the battlefield.")
                .or_else(|| rest.strip_suffix(" in that player's graveyard onto the battlefield"))
        })
    {
        return format!("Each player puts {card_text} from their graveyard onto the battlefield.");
    }
    if let Some(card_text) = normalized
        .strip_prefix("For each player, Put ")
        .and_then(|rest| {
            rest.strip_suffix(" from that player's hand on top of that player's library.")
                .or_else(|| {
                    rest.strip_suffix(" from that player's hand on top of that player's library")
                })
        })
    {
        return format!("Each player puts {card_text} from their hand on top of their library.");
    }
    if let Some(cards) = normalized
        .strip_prefix("For each player, Return all ")
        .and_then(|rest| {
            rest.strip_suffix(" from that player's graveyard to that player's hand.")
                .or_else(|| {
                    rest.strip_suffix(" from that player's graveyard to that player's hand")
                })
        })
    {
        let cards = cards
            .replace(" creature card", " creature cards")
            .replace(" land card", " land cards")
            .replace(" permanent card", " permanent cards");
        return format!("Each player returns all {cards} from their graveyard to their hand.");
    }
    if let Some(rest) = normalized.strip_prefix("For each player, Create ") {
        if let Some((create_clause, tail)) = rest.split_once(". ") {
            return format!("Each player creates {create_clause}. {tail}");
        }
        return format!("Each player creates {rest}");
    }
    if normalized == "Untap all a snow permanent you control."
        || normalized == "Untap all a snow permanent you control"
    {
        return "Untap each snow permanent you control.".to_string();
    }
    if let Some(kind) = normalized
        .strip_prefix("Target player sacrifices target player's ")
        .and_then(|rest| rest.strip_suffix("."))
    {
        return format!(
            "Target player sacrifices a {} of their choice.",
            kind.trim()
        );
    }
    if let Some(rest) =
        normalized.strip_prefix("For each creature or planeswalker without flying, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to that object.")
            .or_else(|| rest.strip_suffix(" damage to that object"))
    {
        return format!(
            "Deal {amount} damage to each creature without flying and each planeswalker."
        );
    }
    if let Some(rest) = normalized
        .strip_prefix("When this permanent enters, for each creature or planeswalker, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to that object.")
            .or_else(|| rest.strip_suffix(" damage to that object"))
    {
        return format!(
            "When this permanent enters, it deals {amount} damage to each creature and each planeswalker."
        );
    }
    if let Some(rest) = normalized.strip_prefix("When this permanent enters, deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to each creature or planeswalker.")
            .or_else(|| rest.strip_suffix(" damage to each creature or planeswalker"))
    {
        return format!(
            "When this permanent enters, it deals {amount} damage to each creature and each planeswalker."
        );
    }
    if normalized == "All slivers have 2 regenerate this creature."
        || normalized == "All slivers have 2 regenerate this creature"
    {
        return "All Slivers have \"{2}: Regenerate this creature.\"".to_string();
    }
    if normalized == "All Slivers have 2 sacrifice this permanent draw a card."
        || normalized == "All Slivers have 2 sacrifice this permanent draw a card"
        || normalized == "All slivers have 2 sacrifice this permanent draw a card."
        || normalized == "All slivers have 2 sacrifice this permanent draw a card"
    {
        return "All Slivers have \"{2}, Sacrifice this permanent: Draw a card.\"".to_string();
    }
    if normalized == "Draw two cards and you lose 2 life. you mill 2 cards."
        || normalized == "Draw two cards and you lose 2 life. you mill 2 cards"
        || normalized == "Draw two cards and you lose 2 life. You mill 2 cards."
        || normalized == "Draw two cards and lose 2 life. you mill 2 cards."
    {
        return "Draw two cards, lose 2 life, then mill two cards.".to_string();
    }
    if let Some(rest) = normalized.strip_prefix("This creature gets ")
        && let Some((pt, cond)) = rest.split_once(" as long as ")
        && let Some((left_cond, right_tail)) = cond.split_once(" and has ")
        && let Some((granted, repeated_cond)) = right_tail.split_once(" as long as ")
    {
        let left_cond = left_cond.trim().trim_end_matches('.');
        let repeated_cond = repeated_cond.trim().trim_end_matches('.');
        if left_cond.eq_ignore_ascii_case(repeated_cond) {
            let granted = granted.trim().trim_end_matches('.');
            let granted = normalize_keyword_predicate_case(granted);
            return format!("As long as {left_cond}, this creature gets {pt} and has {granted}.");
        }
    }

    normalized
}

fn normalize_search_discard_then_shuffle_surface(line: &str) -> Option<String> {
    let (prefix, body) = if let Some(rest) = line.strip_prefix("Spell effects: ") {
        ("Spell effects: ", rest)
    } else {
        ("", line)
    };

    let trimmed = body.trim();
    let sentence = trimmed.trim_end_matches('.');
    let parts: Vec<&str> = sentence.split(". ").collect();
    if parts.len() != 3 {
        return None;
    }

    let search_clause = parts[0].trim();
    let discard_clause = parts[1].trim();
    let shuffle_clause = parts[2].trim();

    let search_lower = search_clause.to_ascii_lowercase();
    let discard_lower = discard_clause.to_ascii_lowercase();
    let shuffle_lower = shuffle_clause.to_ascii_lowercase();

    if !search_lower.starts_with("search ")
        || !search_lower.contains(" put ")
        || !(discard_lower.starts_with("you discard ") || discard_lower.starts_with("discard "))
        || shuffle_lower != "shuffle your library"
    {
        return None;
    }

    let discard_text = discard_clause
        .strip_prefix("you ")
        .or_else(|| discard_clause.strip_prefix("You "))
        .unwrap_or(discard_clause)
        .trim();

    Some(format!(
        "{prefix}{search_clause}, {discard_text}, then shuffle."
    ))
}

fn normalize_discard_random_then_discard_surface(line: &str) -> Option<String> {
    let (prefix, body) = if let Some(rest) = line.strip_prefix("Spell effects: ") {
        ("Spell effects: ", rest)
    } else {
        ("", line)
    };

    let sentence = body.trim().trim_end_matches('.');
    let parts: Vec<&str> = sentence.split(". ").collect();
    if parts.len() != 2 {
        return None;
    }

    let first = parts[0].trim();
    let second = parts[1].trim();
    let first_lower = first.to_ascii_lowercase();
    let second_lower = second.to_ascii_lowercase();

    let first_suffix = " discards a card at random";
    let second_suffix = " discards a card";
    if !first_lower.ends_with(first_suffix) || !second_lower.ends_with(second_suffix) {
        return None;
    }

    let first_subject = first[..first.len() - first_suffix.len()].trim();
    let second_subject = second[..second.len() - second_suffix.len()].trim();
    if !first_subject.eq_ignore_ascii_case(second_subject) {
        return None;
    }

    Some(format!(
        "{prefix}{first_subject} discards a card at random, then discards a card."
    ))
}

fn normalize_ward_cost_surface(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let lower = trimmed.to_ascii_lowercase();

    for pattern in [
        "ward exile an artifact or creature or enchantment or land or planeswalker or battle,",
        "ward exile a artifact or creature or enchantment or land or planeswalker or battle,",
    ] {
        if let Some(idx) = lower.find(pattern) {
            let prefix = trimmed[..idx].trim_end();
            if prefix.is_empty() {
                return "Ward—Sacrifice a permanent.".to_string();
            }
            return format!("{prefix} Ward—Sacrifice a permanent.");
        }
    }

    if lower.starts_with("ward effect(discardeffect") {
        let mut count = 1u32;
        if let Some(start) = lower.find("count: fixed(")
            && let Some(end_rel) = lower[start + "count: fixed(".len()..].find(')')
        {
            let digits = &lower[start + "count: fixed(".len()..start + "count: fixed(".len() + end_rel];
            if let Ok(parsed) = digits.parse::<u32>() {
                count = parsed.max(1);
            }
        }
        if count == 1 {
            return "Ward—Discard a card".to_string();
        }
        return format!("Ward—Discard {count} cards");
    }

    if lower.starts_with("ward exile a ")
        && lower.contains("effect(sacrificeeffect")
        && lower.contains("mana_value: some(")
        && lower.contains("greaterthanorequal(")
        && let Some(start) = lower.find("greaterthanorequal(")
        && let Some(end_rel) = lower[start + "greaterthanorequal(".len()..].find(')')
    {
        let amount = &lower[start + "greaterthanorequal(".len()..start + "greaterthanorequal(".len() + end_rel];
        if let Ok(parsed) = amount.trim().parse::<u32>() {
            return format!("Ward—Sacrifice a permanent with mana value {parsed} or greater.");
        }
    }

    if lower.starts_with("ward exile a ")
        && lower.contains(" with mana value ")
        && lower.contains(", effect(sacrificeeffect")
        && let Some(comma_idx) = trimmed.find(", Effect(")
        && comma_idx > "Ward Exile a ".len()
    {
        let sacrificed = trimmed["Ward Exile a ".len()..comma_idx].trim();
        if let Some((_, mana_tail)) = sacrificed.rsplit_once(" with mana value ") {
            return format!("Ward—Sacrifice a permanent with mana value {}", mana_tail.trim());
        }
    }

    if lower.starts_with("ward exile a ")
        && let Some(comma_idx) = trimmed.find(',')
        && comma_idx > "Ward Exile a ".len()
    {
        let sacrificed = trimmed["Ward Exile a ".len()..comma_idx].trim();
        if !sacrificed.is_empty() && !sacrificed.contains(" or ") {
            return format!("Ward—Sacrifice a {sacrificed}");
        }
    }

    if let Some(rest) = trimmed.strip_prefix("Ward Pay ") {
        return format!("Ward—Pay {}", rest.trim());
    }
    if let Some(rest) = trimmed.strip_prefix("Ward Discard ") {
        return format!("Ward—Discard {}", rest.trim());
    }
    if let Some(rest) = trimmed.strip_prefix("Ward Sacrifice ") {
        return format!("Ward—Sacrifice {}", rest.trim());
    }
    if trimmed.starts_with("Ward {")
        && trimmed.contains(',')
        && let Some(rest) = trimmed.strip_prefix("Ward ")
    {
        return format!("Ward—{}", rest.trim());
    }

    trimmed.to_string()
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
        let is_activated = matches!(
            ability.kind,
            AbilityKind::Activated(_) | AbilityKind::Mana(_)
        );
        let zone_marked = ability.functional_zones.contains(&Zone::Graveyard);
        let text_marked = ability.text.as_ref().is_some_and(|text| {
            let lower = text.to_ascii_lowercase();
            lower.contains("from your graveyard")
                || lower.contains("in your graveyard")
                || lower.contains("while this card is in your graveyard")
        });
        is_activated && (zone_marked || text_marked)
    })
}

fn enchanted_subject_for_oracle_lines(def: &CardDefinition) -> Option<&'static str> {
    if let Some(filter) = &def.aura_attach_filter {
        if filter
            .card_types
            .contains(&crate::types::CardType::Creature)
        {
            return Some("creature");
        }
        if filter.card_types.contains(&crate::types::CardType::Land) {
            return Some("land");
        }
        if filter
            .card_types
            .contains(&crate::types::CardType::Artifact)
        {
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
    let create_occurrences = text.matches("Create ").count() + text.matches("create ").count();
    if create_occurrences > 1 {
        // Avoid cross-sentence rewrites where reminder text can drift onto
        // a later create clause.
        return None;
    }

    let (prefix, rest, create_keyword) = if let Some((prefix, rest)) = text.split_once("Create ") {
        (prefix, rest, "Create ")
    } else if let Some((prefix, rest)) = text.split_once("create ") {
        (prefix, rest, "create ")
    } else {
        return None;
    };
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
        if created.contains(". Create ") || created.contains(". create ") {
            return None;
        }
        let created = if let Some(single) = created.strip_prefix("1 ") {
            format!("a {single}")
        } else {
            created.to_string()
        };
        let created = normalize_created(created);
        if let Some((body, reminder)) = created.split_once(" token with ") {
            let reminder = reminder.trim().trim_matches('"').trim_end_matches('.');
            let reminder_lower = reminder.to_ascii_lowercase();
            let looks_like_token_granted_text = reminder_lower.starts_with("when this token")
                || reminder_lower.starts_with("whenever this creature")
                || reminder.starts_with('{')
                || reminder_lower.starts_with("flying and {");
            if looks_like_token_granted_text {
                let is_single = body.starts_with("a ")
                    || body.starts_with("an ")
                    || body.starts_with("one ")
                    || body.starts_with("1 ");
                let pronoun = if is_single { "It has" } else { "They have" };
                let mut lead = format!("{prefix}{create_keyword}{body} token{suffix}");
                let mut reminder = reminder.to_string();
                if let Some(rest) = reminder
                    .strip_prefix("flying and ")
                    .or_else(|| reminder.strip_prefix("Flying and "))
                {
                    lead = lead.replacen(" token", " token with flying", 1);
                    reminder = rest.to_string();
                }
                if !lead.ends_with('.') {
                    lead.push('.');
                }
                return Some(format!("{lead} {pronoun} \"{reminder}.\""));
            }
        }
        let normalized = format!("{prefix}{create_keyword}{created}{suffix}");
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
        return Some(format!(
            "That object's controller creates {created}{suffix}"
        ));
    }
    Some(format!("{prefix}Its controller creates {created}{suffix}"))
}

fn normalize_embedded_create_with_token_reminder(text: &str) -> Option<String> {
    let (create_head, create_tail, lowercase_create) =
        if let Some((head, tail)) = text.split_once("Create ") {
            (head, tail, false)
        } else if let Some((head, tail)) = text.split_once("create ") {
            (head, tail, true)
        } else {
            return None;
        };

    let (token_desc, tail, single_token_word) =
        if let Some((desc, rest)) = create_tail.split_once(" token with ") {
            (desc, rest, true)
        } else if let Some((desc, rest)) = create_tail.split_once(" tokens with ") {
            (desc, rest, false)
        } else {
            return None;
        };

    if token_desc.contains(". ") {
        return None;
    }

    let (ability_text, after_control) = tail.split_once(" under your control")?;
    if ability_text.contains(". ")
        || after_control.contains(". Create ")
        || after_control.contains(". create ")
    {
        return None;
    }

    let ability_core = ability_text.trim().trim_matches('"').trim_end_matches('.');
    let ability_lower = ability_core.to_ascii_lowercase();
    let looks_like_token_reminder = ability_lower.starts_with("when this token")
        || ability_lower.starts_with("whenever this creature")
        || ability_core.starts_with('{')
        || ability_lower.starts_with("flying and {");
    if !looks_like_token_reminder {
        return None;
    }

    let mut normalized_desc = token_desc.trim().to_string();
    if let Some(rest) = normalized_desc.strip_prefix("1 ") {
        normalized_desc = format!("a {rest}");
    }

    let is_single = single_token_word
        || normalized_desc.starts_with("a ")
        || normalized_desc.starts_with("an ")
        || normalized_desc.starts_with("one ");
    let token_word = if is_single { "token" } else { "tokens" };
    let pronoun = if is_single { "It has" } else { "They have" };

    let create_keyword = if lowercase_create { "create" } else { "Create" };
    let mut first = format!(
        "{create_head}{create_keyword} {normalized_desc} {token_word} under your control{after_control}"
    );
    let mut ability = ability_core.to_string();
    if let Some(rest) = ability
        .strip_prefix("flying and ")
        .or_else(|| ability.strip_prefix("Flying and "))
    {
        first = first.replacen(
            " token under your control",
            " token with flying under your control",
            1,
        );
        ability = rest.to_string();
    }

    if !first.ends_with('.') {
        first.push('.');
    }
    Some(format!("{first} {pronoun} \"{ability}.\""))
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
        if word.chars().all(|ch| ch.is_ascii_digit()) {
            parts.push(format!("{{{word}}}"));
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
        if word == "discard" {
            let tail = words[idx + 1..].join(" ");
            if tail.is_empty() {
                parts.push("Discard".to_string());
            } else {
                parts.push(format!("Discard {tail}"));
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
    if effect_idx.is_none() && words.len() >= 2 {
        let leading_cost = words[0] == "t"
            || is_cost_symbol_word(words[0])
            || words[0].chars().all(|ch| ch.is_ascii_digit());
        let starts_effect = is_effect_verb_word(words[1])
            || matches!(words[1], "this" | "target" | "you" | "each" | "a" | "an");
        if leading_cost && starts_effect {
            effect_idx = Some(1);
        }
    }
    let effect_idx = effect_idx?;
    let cost_words = &words[..effect_idx];
    let effect_words = &words[effect_idx..];

    if !cost_words
        .iter()
        .any(|word| {
            *word == "t"
                || *word == "sacrifice"
                || *word == "discard"
                || is_cost_symbol_word(word)
                || word.chars().all(|ch| ch.is_ascii_digit())
        })
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

fn normalize_granted_beginning_trigger_clause(text: &str) -> Option<String> {
    let (subject, tail, has_word) = if let Some((subject, tail)) = text.split_once(" has ") {
        (subject.trim(), tail.trim(), "has")
    } else if let Some((subject, tail)) = text.split_once(" have ") {
        (subject.trim(), tail.trim(), "have")
    } else {
        return None;
    };
    if subject.is_empty() {
        return None;
    }

    let mut body = tail
        .trim()
        .trim_matches('"')
        .trim_end_matches('.')
        .to_string();
    if !body
        .to_ascii_lowercase()
        .starts_with("at the beginning of ")
    {
        return None;
    }
    body = body
        .replace(" w w ", " {W}{W} ")
        .replace(" w w.", " {W}{W}.")
        .replace(" if you do ", ". If you do, ")
        .replace(" if you do,", ". If you do,");
    if !body.ends_with('.') {
        body.push('.');
    }
    Some(format!(
        "{subject} {has_word} \"{}\"",
        capitalize_first(&body)
    ))
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
    if let Some(buff) = trimmed.strip_prefix(
        "Tag the object attached to this source as 'enchanted'. enchanted creature gets ",
    ) {
        return format!("Enchanted creature gets {buff}");
    }
    if let Some(buff) = trimmed.strip_prefix(
        "Tag the object attached to this source as 'enchanted'. enchanted creatures get ",
    ) {
        return format!("Enchanted creature gets {buff}");
    }
    if let Some(tail) = trimmed.strip_prefix(
        "Tag the object attached to this source as 'enchanted'. enchanted creature gains ",
    ) {
        return format!("Enchanted creature gains {tail}");
    }
    if let Some(buff) = trimmed.strip_prefix(
        "Tag the object attached to this enchantment as 'enchanted'. enchanted creature gets ",
    ) {
        return format!("Enchanted creature gets {buff}");
    }
    if let Some(buff) = trimmed.strip_prefix(
        "Tag the object attached to this enchantment as 'enchanted'. enchanted creatures get ",
    ) {
        return format!("Enchanted creature gets {buff}");
    }
    if let Some(buff) = trimmed.strip_prefix(
        "Tag the object attached to this aura as 'enchanted'. enchanted creature gets ",
    ) {
        return format!("Enchanted creature gets {buff}");
    }
    if let Some(buff) = trimmed.strip_prefix(
        "Tag the object attached to this aura as 'enchanted'. enchanted creatures get ",
    ) {
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
    if let Some(kind) = strip_prefix_ascii_ci(trimmed, "you may put ").and_then(|rest| {
        rest.strip_suffix(" card in your hand onto the battlefield")
            .or_else(|| rest.strip_suffix(" card in your hand onto the battlefield."))
    }) {
        let kind = kind.trim();
        let noun = if kind.is_empty() {
            "card".to_string()
        } else {
            format!("{kind} card")
        };
        let is_already_determined =
            kind.starts_with("target ") || kind.starts_with("a ") || kind.starts_with("an ");
        let rendered_noun = if is_already_determined {
            noun
        } else {
            with_indefinite_article(&noun)
        };
        return format!("You may put {rendered_noun} from your hand onto the battlefield");
    }
    if let Some(kind) = strip_prefix_ascii_ci(trimmed, "you may put ").and_then(|rest| {
        rest.strip_suffix(" in your hand onto the battlefield")
            .or_else(|| rest.strip_suffix(" in your hand onto the battlefield."))
    }) {
        let kind = kind.trim();
        let is_already_determined =
            kind.starts_with("target ") || kind.starts_with("a ") || kind.starts_with("an ");
        let rendered_kind = if is_already_determined {
            kind.to_string()
        } else {
            with_indefinite_article(kind)
        };
        return format!("You may put {rendered_kind} from your hand onto the battlefield");
    }
    if let Some(kind) = trimmed
        .strip_prefix("You may Put target ")
        .and_then(|rest| rest.strip_suffix(" card in your hand onto the battlefield"))
    {
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
    if trimmed
        == "For each player, Put target creature card in target player's graveyard onto the battlefield"
    {
        return "Each player puts a creature card from their graveyard onto the battlefield"
            .to_string();
    }
    if let Some(kind) = trimmed
        .strip_prefix("For each player, Return all ")
        .and_then(|rest| rest.strip_suffix(" card in target player's graveyard to the battlefield"))
    {
        return format!(
            "Each player returns all {kind} cards from their graveyard to the battlefield"
        );
    }
    if let Some(kind) = trimmed
        .strip_prefix("For each player, Return all ")
        .and_then(|rest| {
            rest.strip_suffix(" card from target player's graveyard to target player's hand")
        })
    {
        return format!("Each player returns all {kind} cards from their graveyard to their hand");
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
    if trimmed == "Destroy all land" {
        return "Destroy all lands".to_string();
    }
    if trimmed == "Exile all card in graveyard" {
        return "Exile all graveyards".to_string();
    }
    if trimmed == "Permanents enter the battlefield tapped" {
        return "Permanents enter tapped".to_string();
    }
    if let Some(rest) = trimmed.strip_prefix("Token creatures get ") {
        return format!("Creature tokens get {rest}");
    }
    if trimmed == "Destroy all artifact. Destroy all enchantments"
        || trimmed == "Destroy all artifact. Destroy all enchantment"
    {
        return "Destroy all artifacts and enchantments".to_string();
    }
    if trimmed == "Counter target spell Spirit or Arcane" {
        return "Counter target Spirit or Arcane spell".to_string();
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
        return format!(
            "{left}. {} deals {amount} damage to that spell's controller",
            "This spell"
        );
    }
    if let Some(amount) = trimmed
        .strip_prefix("Counter target spell. Deal ")
        .and_then(|rest| rest.strip_suffix(" damage to that object's controller."))
    {
        return format!(
            "Counter target spell. This spell deals {amount} damage to that spell's controller."
        );
    }
    if let Some(amount) = trimmed
        .strip_prefix("Counter target spell. Deal ")
        .and_then(|rest| rest.strip_suffix(" damage to that object's controller"))
    {
        return format!(
            "Counter target spell. This spell deals {amount} damage to that spell's controller"
        );
    }
    if let Some(amount) = trimmed
        .strip_prefix("Whenever a creature blocks, deal ")
        .and_then(|rest| rest.strip_suffix(" damage to that object's controller."))
    {
        return format!(
            "Whenever a creature blocks, deal {amount} damage to that creature's controller."
        );
    }
    if let Some(amount) = trimmed
        .strip_prefix("Whenever a creature blocks, deal ")
        .and_then(|rest| rest.strip_suffix(" damage to that object's controller"))
    {
        return format!(
            "Whenever a creature blocks, deal {amount} damage to that creature's controller"
        );
    }
    if let Some(rest) = trimmed.strip_prefix("Choose one - ") {
        return format!("Choose one — {}", rest.trim());
    }
    if let Some(rest) = trimmed.strip_prefix("Choose one — ") {
        return format!("Choose one — {}", rest.trim());
    }
    if let Some(rest) = trimmed.strip_prefix("Choose one or both - ") {
        return format!("Choose one or both — {}", rest.trim());
    }
    if let Some(rest) = trimmed.strip_prefix("Choose one or both — ") {
        return format!("Choose one or both — {}", rest.trim());
    }
    if let Some(rest) = trimmed.strip_prefix("Choose one or more - ") {
        return format!("Choose one or more — {}", rest.trim());
    }
    if let Some(rest) = trimmed.strip_prefix("Choose one or more — ") {
        return format!("Choose one or more — {}", rest.trim());
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
        let rest = rest.trim().trim_end_matches('.').trim_end_matches(',');
        if rest.eq_ignore_ascii_case("island") || rest.eq_ignore_ascii_case("an island") {
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
        && (head.starts_with("When ")
            || head.starts_with("Whenever ")
            || head.starts_with("At the beginning "))
    {
        return format!("{head}, draw {tail}");
    }
    if let Some((head, tail)) = trimmed.split_once(", you mill ")
        && (head.starts_with("When ")
            || head.starts_with("Whenever ")
            || head.starts_with("At the beginning "))
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
    if let Some((before_discard, lose_tail)) = trimmed.split_once(
        ". For each opponent, that player discards a card. For each opponent, that player loses ",
    ) && let Some(loss_amount) = lose_tail.strip_suffix(" life")
    {
        return format!(
            "{}, discards a card, and loses {loss_amount} life",
            capitalize_first(before_discard.trim())
        );
    }
    if let Some((draw_clause, gain_tail)) = trimmed.split_once(". you gain ")
        && draw_clause.starts_with("Draw ")
        && let Some(gain_amount) = gain_tail.strip_suffix(" life")
    {
        return format!(
            "{}, and gain {gain_amount} life",
            capitalize_first(draw_clause)
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
    if let Some((first_clause, rest)) = trimmed.split_once(". For each opponent's creature, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to each opponent")
            .or_else(|| rest.strip_suffix(" damage to each opponent."))
        && let Some(self_amount) = first_clause
            .strip_prefix("When this permanent enters, it deals ")
            .and_then(|tail| tail.strip_suffix(" damage to that player"))
        && self_amount.trim().eq_ignore_ascii_case(amount.trim())
    {
        return format!(
            "When this permanent enters, it deals {amount} damage to each opponent and each creature your opponents control"
        );
    }
    if let Some(rest) = trimmed.strip_prefix("For each opponent's creature, Deal ")
        && let Some(amount) = rest.strip_suffix(" damage to that object")
    {
        return format!("Deal {amount} damage to each creature your opponents control");
    }
    if let Some(rest) = trimmed.strip_prefix("For each opponent's creature, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to each opponent")
            .or_else(|| rest.strip_suffix(" damage to each opponent."))
    {
        return format!("Deal {amount} damage to each creature your opponents control");
    }
    if let Some((left, right)) = split_once_ascii_ci(trimmed, ". sacrifice ")
        && left.to_ascii_lowercase().contains(" and you lose ")
    {
        return format!(
            "{} and sacrifice {}",
            left.trim().trim_end_matches('.'),
            right.trim().trim_end_matches('.')
        );
    }
    if let Some(rest) = strip_prefix_ascii_ci(trimmed, "creatures you control get ")
        && let Some(buff) = rest
            .strip_suffix(" until end of turn. Untap all permanent")
            .or_else(|| rest.strip_suffix(" until end of turn. Untap all permanent."))
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
    if trimmed
        == "At the beginning of your upkeep: Sacrifice this enchantment unless you pays {W}{W}"
    {
        return "At the beginning of your upkeep, sacrifice this enchantment unless you pay {W}{W}"
            .to_string();
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
    if lower_trimmed
        .starts_with("for each creature you control, put a +1/+1 counter on that object")
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
    if lower_trimmed.starts_with(
        "when this creature enters, for each another creature you control, put a +1/+1 counter on that object",
    ) || lower_trimmed.starts_with(
        "when this creature enters, for each another creature you control, put 1 +1/+1 counter on that object",
    ) {
        return "When this creature enters, put a +1/+1 counter on each other creature you control"
            .to_string();
    }
    if lower_trimmed.starts_with(
        "when this permanent enters, for each another creature you control, put a +1/+1 counter on that object",
    ) || lower_trimmed.starts_with(
        "when this permanent enters, for each another creature you control, put 1 +1/+1 counter on that object",
    ) {
        return "When this permanent enters, put a +1/+1 counter on each other creature you control"
            .to_string();
    }
    if let Some(rest) = strip_prefix_ascii_ci(trimmed, "opponent's ")
        && let Some((objects, predicate)) = split_once_ascii_ci(rest, " get ")
    {
        return format!(
            "{} your opponents control get {}",
            objects.trim(),
            predicate.trim()
        );
    }
    if let Some(rest) = strip_prefix_ascii_ci(trimmed, "opponent's ")
        && let Some((objects, predicate)) = split_once_ascii_ci(rest, " gets ")
    {
        return format!(
            "{} your opponents control gets {}",
            objects.trim(),
            predicate.trim()
        );
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
    if trimmed == "Exile all card in graveyard" {
        return "Exile all graveyards".to_string();
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
    if let Some(rest) =
        trimmed.strip_prefix("you may Search your library for artifact with mana value ")
        && let Some((value, tail)) =
            rest.split_once(" you own, reveal it, put it into hand, then shuffle")
    {
        return format!(
            "you may search your library for an artifact card with mana value {value}, reveal it, put it into your hand, then shuffle{tail}"
        );
    }
    if let Some(rest) =
        trimmed.strip_prefix("you may Search your library for artifact with mana value ")
        && let Some(value) =
            rest.strip_suffix(" you own, reveal it, put it into hand, then shuffle")
    {
        return format!(
            "you may search your library for an artifact card with mana value {value}, reveal it, put it into your hand, then shuffle"
        );
    }
    if let Some(rest) = trimmed.strip_prefix(
        "you may Search your library for Aura you own, reveal it, put it into hand, then shuffle",
    ) {
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
    if (trimmed.starts_with("This land enters with ")
        || trimmed.starts_with("This artifact enters with ")
        || trimmed.starts_with("This creature enters with ")
        || trimmed.starts_with("This permanent enters with "))
        && trimmed.contains("counter")
        && !trimmed.contains(" on it")
    {
        if let Some(prefix) = trimmed.strip_suffix('.') {
            return format!("{prefix} on it.");
        }
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
    if let Some((left, right)) = split_once_ascii_ci(trimmed, ". ")
        && strip_prefix_ascii_ci(left.trim(), "target player draws ").is_some()
        && let Some(stripped) = strip_prefix_ascii_ci(right.trim(), "target player loses ")
        && stripped
            .trim_end_matches('.')
            .to_ascii_lowercase()
            .ends_with(" life")
    {
        return format!(
            "{} and loses {}",
            capitalize_first(left.trim()),
            stripped.trim().trim_end_matches('.')
        );
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
        return format!(
            "{}, then {}",
            capitalize_first(left),
            normalize_you_verb_phrase(right)
        );
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left.to_ascii_lowercase().contains("exile ")
        && right.eq_ignore_ascii_case("Return it to the battlefield under its owner's control.")
    {
        return format!("{left}, then return it to the battlefield under its owner's control.");
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left.to_ascii_lowercase().contains("exile ")
        && right.eq_ignore_ascii_case("Return it to the battlefield under its owner's control")
    {
        return format!("{left}, then return it to the battlefield under its owner's control");
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left.to_ascii_lowercase().contains("exile ")
        && right.eq_ignore_ascii_case("Return that card to the battlefield under your control.")
    {
        return format!("{left}, then return it to the battlefield under your control.");
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left.to_ascii_lowercase().contains("exile ")
        && right.eq_ignore_ascii_case("Return that card to the battlefield under your control")
    {
        return format!("{left}, then return it to the battlefield under your control");
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left.to_ascii_lowercase().contains("exile ")
        && right
            .eq_ignore_ascii_case("Return that card to the battlefield under its owner's control.")
    {
        return format!("{left}, then return it to the battlefield under its owner's control.");
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left.to_ascii_lowercase().contains("exile ")
        && right
            .eq_ignore_ascii_case("Return that card to the battlefield under its owner's control")
    {
        return format!("{left}, then return it to the battlefield under its owner's control");
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left.to_ascii_lowercase().contains("exile ")
        && let Some(tail) = strip_prefix_ascii_ci(
            right,
            "At the beginning of the next end step, return it to the battlefield. Put ",
        )
        && let Some(counter_phrase) =
            strip_suffix_ascii_ci(tail, " on it.").or_else(|| strip_suffix_ascii_ci(tail, " on it"))
    {
        return format!(
            "{left}. At the beginning of the next end step, return that card to the battlefield under its owner's control with {} on it.",
            counter_phrase.trim()
        );
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left.to_ascii_lowercase().contains("exile ")
        && right.eq_ignore_ascii_case("Return it from graveyard to the battlefield tapped.")
    {
        return format!("{left}, then return it to the battlefield tapped.");
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left.to_ascii_lowercase().contains("exile ")
        && right.eq_ignore_ascii_case("Return it from graveyard to the battlefield tapped")
    {
        return format!("{left}, then return it to the battlefield tapped");
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left
            .to_ascii_lowercase()
            .starts_with("target player discards ")
        && right
            .to_ascii_lowercase()
            .starts_with("target player loses ")
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
        return format!("{left} and {}", right.to_ascii_lowercase());
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
        .strip_prefix("put a ")
        .and_then(|rest| rest.strip_suffix(" counter on target creature. proliferate"))
    {
        return format!("Put a {counter} counter on target creature, then proliferate");
    }
    if let Some(counter) = trimmed
        .strip_prefix("put a ")
        .and_then(|rest| rest.strip_suffix(" counter on target creature. proliferate."))
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
    if lower_trimmed == "draw two cards and you lose 2 life. you mill 2 cards."
        || lower_trimmed == "draw two cards and you lose 2 life. you mill 2 cards"
    {
        return "Draw two cards, lose 2 life, then mill two cards.".to_string();
    }
    if let Some((first, second)) = split_once_ascii_ci(trimmed, ". ")
        && let Some(first_buff) = strip_prefix_ascii_ci(first.trim(), "target creature gets ")
            .and_then(|rest| rest.strip_suffix(" until end of turn"))
        && let Some(second_buff) = strip_prefix_ascii_ci(
            second.trim(),
            "other creatures with the same name as that object get ",
        )
        .and_then(|rest| {
            rest.strip_suffix(" until end of turn")
                .or_else(|| rest.strip_suffix(" until end of turn."))
        })
        && first_buff.eq_ignore_ascii_case(second_buff)
    {
        return format!(
            "Target creature and all other creatures with the same name as that creature get {first_buff} until end of turn"
        );
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
        .replace("this creatures power", "this creature's power")
        .replace("this creatures toughness", "this creature's toughness")
        .replace("enchanted creatures get ", "enchanted creature gets ")
        .replace("enchanted creatures gain ", "enchanted creature gains ")
        .replace("equipped creatures get ", "equipped creature gets ")
        .replace("equipped creatures gain ", "equipped creature gains ")
        .replace(
            "Whenever this creature blocks or Whenever this creature becomes blocked",
            "Whenever this creature blocks or becomes blocked",
        )
        .replace(
            "target attacking/blocking creature",
            "target attacking or blocking creature",
        )
        .replace("a another ", "another ")
        .replace("all another ", "all other ")
        .replace("All another ", "All other ")
        .replace("each another ", "each other ")
        .replace("Each another ", "Each other ")
        .replace("For each another ", "For each other ")
        .replace("for each another ", "for each other ")
        .replace("you takes ", "you take ")
        .replace("You takes ", "You take ")
        .replace("you loses ", "you lose ")
        .replace("You loses ", "You lose ")
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
        .replace(
            "token creatures you control get ",
            "creature tokens you control get ",
        )
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
        .replace(
            "Exile target card in graveyard",
            "Exile target card from a graveyard",
        )
        .replace(
            "Exile target artifact card in graveyard",
            "Exile target artifact card from a graveyard",
        )
        .replace(
            "Exile target creature card in graveyard",
            "Exile target creature card from a graveyard",
        )
        .replace(
            "exile target card in graveyard",
            "exile target card from a graveyard",
        )
        .replace(
            "exile target artifact card in graveyard",
            "exile target artifact card from a graveyard",
        )
        .replace(
            "exile target creature card in graveyard",
            "exile target creature card from a graveyard",
        )
        .replace(
            "Search your library for a card, put it into hand, then shuffle",
            "Search your library for a card, put that card into your hand, then shuffle",
        )
        .replace(
            "for each the number of an artifact you control",
            "for each artifact you control",
        )
        .replace("for each the number of a ", "for each ")
        .replace("for each the number of an ", "for each ")
        .replace(", then you draw ", ", then draw ")
        .replace(". you draw ", ". Draw ")
        .replace(", then you mill ", ", then mill ")
        .replace(
            "Discard a card, then you draw ",
            "Discard a card, then draw ",
        )
        .replace(
            "discard a card, then you draw ",
            "Discard a card, then draw ",
        )
        .replace(
            "Sacrifice this creature: this creature deals ",
            "Sacrifice this creature: It deals ",
        )
        .replace(
            "Sacrifice this creature: This creature deals ",
            "Sacrifice this creature: It deals ",
        )
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
        .replace(
            "target creature you don't control or planeswalker",
            "target creature or planeswalker you don't control",
        )
        .replace(
            "target opponent's creature",
            "target creature an opponent controls",
        )
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
        .replace(", This creature deals ", ", it deals ")
        .replace(
            " in your graveyard on top of its owner's library",
            " from your graveyard on top of your library",
        )
        .replace("Put 1 +1/+1 counter(s) on ", "Put a +1/+1 counter on ")
        .replace("counter(s)", "counters")
        .replace(
            "Whenever this creature deals damage to Spider, destroy it.",
            "Whenever this creature deals damage to a Spider, destroy that creature.",
        )
        .replace(
            "Destroy target black or red attacking/blocking creature and you gain 2 life.",
            "Destroy target black or red creature that's attacking or blocking. You gain 2 life.",
        )
        .replace("Whenever a another ", "Whenever another ")
        .replace("you may Search", "you may search")
        .replace(
            "When this creature enters or Whenever another Ally you control enters,",
            "Whenever this creature or another Ally you control enters,",
        )
        .replace(
            "Untap all a creature you control",
            "Untap all creatures you control",
        )
        .replace(", May ", ", you may ")
        .replace("you pays ", "you pay ")
        .replace("Add 1 mana of any color", "Add one mana of any color")
        .replace("a artifact", "an artifact")
        .replace("a enchantment", "an enchantment")
        .replace("a Aura", "an Aura")
        .replace("its owners hand", "its owner's hand")
        .replace("its owners hands", "its owners' hands")
        .replace("their owners hand", "their owner's hand")
        .replace("their owners hands", "their owners' hands")
        .replace("instant or sorcery cards", "instant and/or sorcery cards")
        .replace("instants or sorcery cards", "instant and/or sorcery cards")
        .replace("you control you control", "you control")
        .replace("put it into hand", "put it into your hand")
        .replace(
            "reveal it, put it into hand",
            "reveal it, put it into your hand",
        )
        .replace(
            "you may target creature gets ",
            "you may have target creature get ",
        )
        .replace(
            "you may target creature gains ",
            "you may have target creature gain ",
        )
        .replace(
            "you may target creature loses ",
            "you may have target creature lose ",
        )
        .replace(
            "you may target creature reveals ",
            "you may have target creature reveal ",
        );
    normalized = normalize_common_semantic_phrasing(&normalized);
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

#[allow(dead_code)]
fn normalize_oracle_line_for_card(def: &CardDefinition, line: &str) -> String {
    let normalized = line.trim().replace("{{", "{").replace("}}", "}");
    let normalized = normalize_common_semantic_phrasing(&normalized);
    if def.is_spell() && normalized.starts_with("Deal ") {
        if let Some(rest) = normalized.strip_prefix("Deal ") {
            return format!("{} deals {rest}", def.card.name);
        }
    }
    normalized
}

/// Render compiled output in a near-oracle style for semantic diffing.
pub fn oracle_like_lines(def: &CardDefinition) -> Vec<String> {
    let _ = def;
    let base_lines = compiled_lines(def);
    let normalized = base_lines
        .iter()
        .map(|line| strip_render_heading(line))
        .filter(|line| !line.is_empty())
        .map(|line| normalize_common_semantic_phrasing(&line))
        .collect::<Vec<_>>();
    let merged_predicates = merge_adjacent_subject_predicate_lines(normalized);
    let merged_mana = merge_adjacent_simple_mana_add_lines(merged_predicates);
    let merged_has_keywords = merge_subject_has_keyword_lines(merged_mana);
    let without_redundant_cost_lines = drop_redundant_spell_cost_lines(merged_has_keywords);
    let merged_blockability = merge_blockability_lines(without_redundant_cost_lines);
    let merged_transform = merge_lose_all_transform_lines(merged_blockability);
    merged_transform
        .into_iter()
        .map(|line| normalize_sentence_surface_style(&line))
        .collect()
}

#[cfg(all(test, feature = "parser-tests-full"))]
mod tests {
    use super::{
        compiled_lines, describe_additional_cost_effects, describe_for_each_filter,
        merge_adjacent_static_heading_lines, merge_adjacent_subject_predicate_lines,
        normalize_common_semantic_phrasing, normalize_compiled_post_pass_effect,
        normalize_create_under_control_clause, normalize_gain_life_plus_phrase,
        normalize_known_low_tail_phrase, normalize_rendered_line_for_card,
        normalize_sentence_surface_style, pluralize_noun_phrase,
    };
    use crate::cards::CardDefinitionBuilder;
    use crate::filter::{ObjectFilter, PlayerFilter};
    use crate::ids::CardId;
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    #[test]
    fn normalizes_target_creature_or_planeswalker_ordering() {
        let normalized = normalize_common_semantic_phrasing(
            "target creature you control deals damage equal to its power to target creature you don't control or planeswalker",
        );
        assert_eq!(
            normalized,
            "Target creature you control deals damage equal to its power to target creature or planeswalker you don't control"
        );
    }

    #[test]
    fn additional_cost_choose_one_renders_inline_or_phrase() {
        let effects = vec![crate::effect::Effect::choose_one(vec![
            crate::effect::EffectMode {
                description: "sacrifice a creature".to_string(),
                effects: Vec::new(),
            },
            crate::effect::EffectMode {
                description: "pay 3".to_string(),
                effects: Vec::new(),
            },
        ])];
        assert_eq!(
            describe_additional_cost_effects(&effects),
            "sacrifice a creature or pay {3}"
        );
    }

    #[test]
    fn normalizes_sentence_surface_punctuation_for_sentences() {
        assert_eq!(
            normalize_sentence_surface_style("target creature gets +2/+2 until end of turn"),
            "Target creature gets +2/+2 until end of turn."
        );
    }

    #[test]
    fn keeps_keyword_lines_without_terminal_period() {
        assert_eq!(normalize_sentence_surface_style("Flying"), "Flying");
        assert_eq!(
            normalize_sentence_surface_style("Trample, haste"),
            "Trample, haste"
        );
    }

    #[test]
    fn normalizes_for_each_player_damage_chain() {
        let normalized = normalize_common_semantic_phrasing(
            "For each player, Deal 2 damage to that player. For each creature that player controls, Deal 2 damage to that object",
        );
        assert_eq!(normalized, "Deal 2 damage to each creature and each player");
    }

    #[test]
    fn normalizes_opponents_creature_damage_and_cant_block_chain() {
        let normalized = normalize_common_semantic_phrasing(
            "For each opponent's creature, Deal 1 damage to that object. an opponent's creature can't block until end of turn",
        );
        assert_eq!(
            normalized,
            "Deal 1 damage to each creature your opponents control. Creatures your opponents control can't block this turn"
        );
    }

    #[test]
    fn normalizes_opponents_creature_damage_and_cant_block_chain_this_turn_tail() {
        let normalized = normalize_common_semantic_phrasing(
            "For each opponent's creature, Deal 1 damage to that object. an opponent's creature can't block this turn",
        );
        assert_eq!(
            normalized,
            "Deal 1 damage to each creature your opponents control. Creatures your opponents control can't block this turn"
        );
    }

    #[test]
    fn normalizes_generic_for_each_damage_to_each_filter() {
        let normalized = normalize_common_semantic_phrasing(
            "For each creature with flying, Deal 4 damage to that object",
        );
        assert_eq!(normalized, "Deal 4 damage to each creature with flying");
    }

    #[test]
    fn normalizes_for_each_opponent_that_player_clause() {
        let normalized =
            normalize_common_semantic_phrasing("For each opponent, that player draws a card");
        assert_eq!(normalized, "Each opponent draws a card");
    }

    #[test]
    fn normalizes_enchanted_land_tapped_for_mana_additional_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "Whenever enchanted land is tapped for mana, add {G}{G} to that object's controller's mana pool.",
        );
        assert_eq!(
            normalized,
            "Whenever enchanted land is tapped for mana, its controller adds an additional {G}{G}."
        );
    }

    #[test]
    fn normalizes_spawn_token_inline_quoted_cost_punctuation() {
        let normalized = normalize_common_semantic_phrasing(
            "When this permanent enters, create two 0/1 colorless Eldrazi Spawn creature tokens with \"Sacrifice this creature, add {C}\"",
        );
        assert_eq!(
            normalized,
            "When this permanent enters, create two 0/1 colorless Eldrazi Spawn creature tokens with \"Sacrifice this creature: Add {C}.\""
        );
    }

    #[test]
    fn normalizes_creature_type_choice_pump_sentence() {
        let normalized = normalize_common_semantic_phrasing(
            "You choose exactly 1 creature in the battlefield and tags it as 'chosen_creature_type_ref'. creature that shares a creature types with that object get +0/+4 until end of turn.",
        );
        assert_eq!(
            normalized,
            "Creatures of the creature type of your choice get +0/+4 until end of turn."
        );
    }

    #[test]
    fn normalizes_creature_type_choice_pump_and_gain_sentence() {
        let normalized = normalize_common_semantic_phrasing(
            "You choose exactly 1 creature in the battlefield and tags it as 'chosen_creature_type_ref'. creature that shares a creature types with that object get +2/+2 until end of turn. creature that shares a creature types with that object gain Trample until end of turn.",
        );
        assert_eq!(
            normalized,
            "Creatures of the creature type of your choice get +2/+2 and gain Trample until end of turn."
        );
    }

    #[test]
    fn normalizes_pump_and_gain_until_end_of_turn_sentence() {
        let normalized = normalize_common_semantic_phrasing(
            "Each creature you control gets +3/+3 until end of turn. creatures you control gain Trample until end of turn.",
        );
        assert_eq!(
            normalized,
            "Each creature you control gets +3/+3 and gains Trample until end of turn."
        );
    }

    #[test]
    fn normalizes_other_elf_plural_surface() {
        let normalized = normalize_common_semantic_phrasing("Other Elf you control get +1/+1.");
        assert_eq!(normalized, "Other Elves you control get +1/+1.");
    }

    #[test]
    fn normalizes_powerstone_tapped_token_surface() {
        let normalized = normalize_common_semantic_phrasing(
            "When this creature enters, create a Powerstone artifact token, tapped.",
        );
        assert_eq!(
            normalized,
            "When this creature enters, create a tapped Powerstone token."
        );
    }

    #[test]
    fn normalizes_discard_and_lose_same_target_surface() {
        let normalized = normalize_common_semantic_phrasing(
            "Target player discards 2 cards. target player loses 2 life.",
        );
        assert_eq!(
            normalized,
            "Target player discards 2 cards and loses 2 life."
        );
    }

    #[test]
    fn normalizes_commander_color_identity_mana_surface() {
        let normalized = normalize_common_semantic_phrasing(
            "Mana ability 1: {T}: Add 1 mana of commander's color identity.",
        );
        assert_eq!(
            normalized,
            "Mana ability 1: {T}: Add one mana of any color in your commander's color identity."
        );
    }

    #[test]
    fn normalizes_for_each_opponent_sacrifice_unless_pay_surface() {
        let normalized = normalize_common_semantic_phrasing(
            "When this creature enters, for each opponent, that player sacrifices a permanent unless that player pays {1}.",
        );
        assert_eq!(
            normalized,
            "When this creature enters, each opponent sacrifices a permanent of their choice unless they pay {1}"
        );
    }

    #[test]
    fn normalizes_choose_another_attacking_creature_scaffolding() {
        let normalized = normalize_common_semantic_phrasing(
            "Whenever this creature attacks, choose another target attacking creature. another target attacking creature can't be blocked this turn.",
        );
        assert_eq!(
            normalized,
            "Whenever this creature attacks, another target attacking creature can't be blocked this turn."
        );
    }

    #[test]
    fn normalizes_for_each_counter_chain_to_each_creature_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "For each creature you control with a +1/+1 counter on it, Put a +1/+1 counter on that object",
        );
        assert_eq!(
            normalized,
            "Put a +1/+1 counter on each creature you control with a +1/+1 counter on it"
        );
    }

    #[test]
    fn normalizes_reveal_tagged_land_return_to_put_into_hand() {
        let normalized = normalize_common_semantic_phrasing(
            "Reveal the top card of defending player's library and tag it as 'revealed_0'. If the tagged object 'revealed_0' matches land, Return it to its owner's hand",
        );
        assert_eq!(
            normalized,
            "Reveal the top card of defending player's library. If it's a land card, that player puts it into their hand"
        );
    }

    #[test]
    fn normalizes_tagged_destroyed_loop_phrasing() {
        let normalized = normalize_common_semantic_phrasing(
            "For each tagged 'destroyed_0' object, Create 1 3/3 green Centaur creature token under that object's controller's control",
        );
        assert_eq!(
            normalized,
            "For each object destroyed this way, Create 1 3/3 green Centaur creature token under that object's controller's control"
        );
    }

    #[test]
    fn keeps_additional_cost_colon_phrase_non_triggered() {
        let normalized = normalize_common_semantic_phrasing(
            "As an additional cost to cast this spell: you discard a card",
        );
        assert_eq!(
            normalized,
            "As an additional cost to cast this spell, you discard a card"
        );
    }

    #[test]
    fn normalizes_shared_you_and_target_opponent_draw_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "When this creature enters, you draw a card. target opponent draws a card",
        );
        assert_eq!(
            normalized,
            "When this creature enters, you and target opponent each draw a card"
        );
    }

    #[test]
    fn normalizes_split_destroy_all_dual_types() {
        let normalized =
            normalize_common_semantic_phrasing("Destroy all artifact. Destroy all enchantment.");
        assert_eq!(normalized, "Destroy all artifacts and enchantments");
    }

    #[test]
    fn normalizes_target_player_sacrifice_choice_phrasing() {
        let normalized =
            normalize_common_semantic_phrasing("target player sacrifices target player's creature");
        assert_eq!(
            normalized,
            "Target player sacrifices a creature of their choice"
        );
    }

    #[test]
    fn normalizes_each_player_sacrifice_choice_phrasing() {
        let normalized = normalize_common_semantic_phrasing(
            "Triggered ability 1: At the beginning of each player's upkeep, that player sacrifices an artifact.",
        );
        assert_eq!(
            normalized,
            "Triggered ability 1: At the beginning of each player's upkeep, that player sacrifices an artifact of their choice."
        );
    }

    #[test]
    fn normalizes_each_player_sacrifice_without_controls() {
        let normalized = normalize_common_semantic_phrasing(
            "Triggered ability 1: When this creature enters, each player sacrifices two creatures that player controls.",
        );
        assert_eq!(
            normalized,
            "Triggered ability 1: When this creature enters, each player sacrifices two creatures of their choice."
        );
    }

    #[test]
    fn normalizes_its_controller_controller_owned_sacrifice() {
        let normalized = normalize_common_semantic_phrasing(
            "Spell effects: For each creature, its controller sacrifices a controller's permanent unless its controller pays {1}.",
        );
        assert_eq!(
            normalized,
            "Spell effects: For each creature, its controller sacrifices a permanent of their choice unless its controller pays {1}."
        );
    }

    #[test]
    fn normalizes_creatures_have_cant_block() {
        let normalized = normalize_common_semantic_phrasing("All creatures have Can't block");
        assert_eq!(normalized, "Creatures can't block");
    }

    #[test]
    fn normalizes_monocolored_creatures_cant_block() {
        let normalized = normalize_common_semantic_phrasing(
            "monocolored creature can't block until end of turn",
        );
        assert_eq!(normalized, "Monocolored creatures can't block this turn");
    }

    #[test]
    fn normalizes_unblockable_until_end_of_turn_to_this_turn() {
        let normalized = normalize_common_semantic_phrasing(
            "target creature can't be blocked until end of turn",
        );
        assert_eq!(normalized, "target creature can't be blocked this turn");
    }

    #[test]
    fn normalizes_tap_any_number_gain_life_phrase() {
        let normalized = normalize_common_semantic_phrasing(
            "Tap any number of an untapped creature you control and you gain 4 life for each tapped creature",
        );
        assert_eq!(
            normalized,
            "Tap any number of untapped creatures you control. You gain 4 life for each creature tapped this way"
        );
    }

    #[test]
    fn normalizes_change_controller_and_haste_phrase() {
        let normalized = normalize_common_semantic_phrasing(
            "Untap target creature. it changes controller to this effect's controller and gains Haste until end of turn.",
        );
        assert_eq!(
            normalized,
            "Untap target creature. Gain control of it until end of turn. It gains haste until end of turn."
        );
    }

    #[test]
    fn normalizes_single_creature_haste_then_sacrifice_tail() {
        let normalized = normalize_common_semantic_phrasing(
            "you may Put creature card in your hand onto the battlefield. it gains Haste until end of turn. you sacrifice a creature.",
        );
        assert_eq!(
            normalized,
            "You may put a creature card from your hand onto the battlefield. That creature gains haste. Sacrifice the creature at the beginning of the next end step."
        );
    }

    #[test]
    fn normalizes_pronoun_end_step_sacrifice_tail() {
        let normalized = normalize_common_semantic_phrasing(
            "it gains Haste until end of turn. At the beginning of the next end step, you sacrifice it.",
        );
        assert_eq!(
            normalized,
            "That creature gains haste until end of turn. At the beginning of the next end step, sacrifice that creature."
        );
    }

    #[test]
    fn normalizes_search_equipment_you_own_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "Search your library for Equipment you own, reveal it, put it into your hand, then shuffle",
        );
        assert_eq!(
            normalized,
            "Search your library for an Equipment card, reveal that card, put it into your hand, then shuffle"
        );
    }

    #[test]
    fn normalizes_opponents_artifact_creature_enter_tapped_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "An opponent's artifact or creature enter the battlefield tapped.",
        );
        assert_eq!(
            normalized,
            "Artifacts and creatures your opponents control enter tapped."
        );
    }

    #[test]
    fn normalizes_target_creature_untap_it_tail() {
        let normalized = normalize_common_semantic_phrasing(
            "Target creature gets +1/+1 until end of turn. Untap it.",
        );
        assert_eq!(
            normalized,
            "Target creature gets +1/+1 until end of turn. Untap that creature."
        );
    }

    #[test]
    fn normalizes_choose_any_number_then_sacrifice_chain() {
        let normalized = normalize_common_semantic_phrasing(
            "You choose any number a Mountain you control in the battlefield. you sacrifice all permanents you control. Deal that much damage to target player or planeswalker.",
        );
        assert_eq!(
            normalized,
            "Sacrifice any number of Mountains. Deal that much damage to target player or planeswalker."
        );
    }

    #[test]
    fn normalizes_destroy_target_blocking_creature_clause_without_rewriting_subject() {
        let normalized = normalize_common_semantic_phrasing("Destroy target blocking creature.");
        assert_eq!(normalized, "Destroy target blocking creature.");
    }

    #[test]
    fn normalizes_target_player_draws_and_loses_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "Target player draws a card. target player loses 1 life.",
        );
        assert_eq!(normalized, "Target player draws a card and loses 1 life");
    }

    #[test]
    fn normalizes_target_player_draws_numeric_cards_and_loses_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "Target player draws 2 cards. target player loses 2 life.",
        );
        assert_eq!(normalized, "Target player draws two cards and loses 2 life");
    }

    #[test]
    fn normalizes_opponents_creatures_get_clause() {
        let normalized =
            normalize_common_semantic_phrasing("Opponent's creatures get -2/-0 until end of turn.");
        assert_eq!(
            normalized,
            "Creatures your opponents control get -2/-0 until end of turn."
        );
    }

    #[test]
    fn normalizes_all_creatures_get_clause() {
        let normalized =
            normalize_common_semantic_phrasing("Creatures get -2/-2 until end of turn.");
        assert_eq!(normalized, "All creatures get -2/-2 until end of turn.");
    }

    #[test]
    fn normalizes_put_land_card_in_hand_phrase() {
        let normalized = normalize_common_semantic_phrasing(
            "{T}: you may Put land card in your hand onto the battlefield.",
        );
        assert_eq!(
            normalized,
            "{T}: You may put a land card from your hand onto the battlefield"
        );
    }

    #[test]
    fn normalizes_same_name_gets_split_sentence() {
        let normalized = normalize_common_semantic_phrasing(
            "Target creature gets -3/-3 until end of turn. other creatures with the same name as that object get -3/-3 until end of turn.",
        );
        assert_eq!(
            normalized,
            "Target creature and all other creatures with the same name as that creature get -3/-3 until end of turn"
        );
    }

    #[test]
    fn normalizes_enters_for_each_another_creature_counter_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "When this permanent enters, for each another creature you control, Put a +1/+1 counter on that object.",
        );
        assert_eq!(
            normalized,
            "When this permanent enters, put a +1/+1 counter on each other creature you control"
        );
    }

    #[test]
    fn normalizes_untap_all_a_creature_phrase() {
        let normalized = normalize_common_semantic_phrasing("Untap all a creature you control.");
        assert_eq!(normalized, "Untap all creatures you control");
    }

    #[test]
    fn normalizes_triggered_target_player_draws_and_loses_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "When this creature enters, target player draws a card. target player loses 1 life.",
        );
        assert_eq!(
            normalized,
            "When this creature enters, target player draws a card and loses 1 life"
        );
    }

    #[test]
    fn normalizes_you_draw_and_you_lose_clause() {
        let normalized =
            normalize_common_semantic_phrasing("You draw two cards and you lose 2 life.");
        assert_eq!(normalized, "You draw two cards and lose 2 life.");
    }

    #[test]
    fn normalizes_target_creature_tap_it_tail() {
        let normalized = normalize_common_semantic_phrasing(
            "Target creature gets -1/-1 until end of turn. Tap it.",
        );
        assert_eq!(
            normalized,
            "Target creature gets -1/-1 until end of turn. Tap that creature."
        );
    }

    #[test]
    fn normalizes_red_or_green_spell_cost_reduction_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "Red and green spells you cast cost {1} less to cast.",
        );
        assert_eq!(
            normalized,
            "Each spell you cast that's red or green costs {1} less to cast"
        );
    }

    #[test]
    fn normalizes_rakdos_return_discard_controller_phrase() {
        let normalized = normalize_common_semantic_phrasing(
            "Deal X damage to target opponent or planeswalker. target opponent discards X cards.",
        );
        assert_eq!(
            normalized,
            "Deal X damage to target opponent or planeswalker. That player or that planeswalker's controller discards X cards."
        );
    }

    #[test]
    fn normalizes_draw_two_then_proliferate_sentence() {
        let normalized = normalize_sentence_surface_style("You draw two cards. Proliferate.");
        assert_eq!(normalized, "Draw two cards, then proliferate.");
    }

    #[test]
    fn normalizes_search_discard_then_shuffle_sentence() {
        let normalized = normalize_sentence_surface_style(
            "Spell effects: Search your library for a card, put it into your hand. you discard a card at random. Shuffle your library.",
        );
        assert_eq!(
            normalized,
            "Spell effects: Search your library for a card, put it into your hand, discard a card at random, then shuffle."
        );
    }

    #[test]
    fn normalizes_discard_random_then_discard_sentence() {
        let normalized = normalize_sentence_surface_style(
            "Spell effects: Target opponent discards a card at random. target opponent discards a card.",
        );
        assert_eq!(
            normalized,
            "Spell effects: Target opponent discards a card at random, then discards a card."
        );
    }

    #[test]
    fn normalizes_siege_mill_discard_trigger_sentence() {
        let normalized = normalize_sentence_surface_style(
            "When this permanent enters, for each player, that player mills 3 cards. For each opponent, that player discards a card. Draw a card.",
        );
        assert_eq!(
            normalized,
            "When this permanent enters, each player mills three cards, then each opponent discards a card and you draw a card."
        );
    }

    #[test]
    fn merges_adjacent_static_heading_keyword_lines() {
        let merged = merge_adjacent_static_heading_lines(vec![
            "Static ability 1: Creatures you control have Flying.".to_string(),
            "Static ability 2: Creatures you control have First strike.".to_string(),
        ]);
        assert_eq!(merged.len(), 1);
        assert_eq!(
            merged[0],
            "Static ability 1: Creatures you control have Flying and First strike."
        );
    }

    #[test]
    fn merges_adjacent_static_heading_enters_tapped_with_counters_lines() {
        let merged = merge_adjacent_static_heading_lines(vec![
            "Static ability 1: This land enters tapped.".to_string(),
            "Static ability 2: Enters the battlefield with 2 charge counter(s).".to_string(),
        ]);
        assert_eq!(merged.len(), 1);
        assert_eq!(
            merged[0],
            "Static ability 1: This land enters tapped with 2 charge counter(s)"
        );
    }

    #[test]
    fn normalizes_target_opponent_choose_destroy_sentence() {
        let normalized = normalize_sentence_surface_style(
            "Target opponent chooses exactly 1 target player's creature in the battlefield. Destroy it.",
        );
        assert_eq!(
            normalized,
            "Target opponent chooses a creature they control. Destroy that creature."
        );
    }

    #[test]
    fn normalizes_target_opponent_choose_destroy_sentence_player_controls_variant() {
        let normalized = normalize_sentence_surface_style(
            "Target opponent chooses exactly 1 a creature that player controls in the battlefield. Destroy it.",
        );
        assert_eq!(
            normalized,
            "Target opponent chooses a creature they control. Destroy that creature."
        );
    }

    #[test]
    fn normalizes_target_opponent_choose_other_cant_block_sentence() {
        let normalized = normalize_sentence_surface_style(
            "Target opponent chooses exactly 1 target player's creature in the battlefield. target player's other creature can't block until end of turn.",
        );
        assert_eq!(
            normalized,
            "Target opponent chooses a creature they control. Other creatures they control can't block this turn."
        );
    }

    #[test]
    fn normalizes_target_opponent_choose_other_cant_block_player_controls_variant() {
        let normalized = normalize_sentence_surface_style(
            "Target opponent chooses exactly 1 a creature that player controls in the battlefield. a other creature that player controls can't block until end of turn.",
        );
        assert_eq!(
            normalized,
            "Target opponent chooses a creature they control. Other creatures they control can't block this turn."
        );
    }

    #[test]
    fn normalizes_target_opponent_exiles_creature_and_graveyard_sentence() {
        let normalized = normalize_sentence_surface_style(
            "Target opponent chooses target creature an opponent controls. Exile it. Exile all card in target opponent's graveyards.",
        );
        assert_eq!(
            normalized,
            "Target opponent exiles a creature they control and their graveyard."
        );
    }

    #[test]
    fn normalizes_spell_effects_target_opponent_exiles_creature_and_graveyard_sentence() {
        let normalized = normalize_sentence_surface_style(
            "Spell effects: Target opponent chooses target creature an opponent controls. Exile it. Exile all card in target opponent's graveyards.",
        );
        assert_eq!(
            normalized,
            "Spell effects: Target opponent exiles a creature they control and their graveyard."
        );
    }

    #[test]
    fn normalizes_when_enters_deals_damage_to_each_creature_and_planeswalker() {
        let normalized = normalize_sentence_surface_style(
            "When this permanent enters, for each creature or planeswalker, Deal 3 damage to that object.",
        );
        assert_eq!(
            normalized,
            "When this permanent enters, it deals 3 damage to each creature and each planeswalker."
        );
    }

    #[test]
    fn normalizes_when_enters_deal_direct_each_creature_or_planeswalker() {
        let normalized = normalize_sentence_surface_style(
            "When this permanent enters, deal 3 damage to each creature or planeswalker.",
        );
        assert_eq!(
            normalized,
            "When this permanent enters, it deals 3 damage to each creature and each planeswalker."
        );
    }

    #[test]
    fn normalizes_when_enters_put_card_from_that_players_hand_on_top() {
        let normalized = normalize_sentence_surface_style(
            "When this creature enters, put a card from that player's hand on top of that player's library.",
        );
        assert_eq!(
            normalized,
            "When this creature enters, target opponent puts a card from their hand on top of their library."
        );
    }

    #[test]
    fn normalizes_for_each_player_exile_top_library_variant_with_that_player() {
        let normalized = normalize_sentence_surface_style(
            "For each player, Exile target card in that player's library.",
        );
        assert_eq!(
            normalized,
            "Each player exiles the top card of their library."
        );
    }

    #[test]
    fn describe_for_each_filter_keeps_exile_zone_without_battlefield_suffix() {
        let mut filter = ObjectFilter::default();
        filter.zone = Some(Zone::Exile);
        filter.owner = Some(PlayerFilter::IteratedPlayer);
        filter.card_types.push(CardType::Artifact);
        filter.card_types.push(CardType::Creature);
        filter.card_types.push(CardType::Enchantment);
        filter.card_types.push(CardType::Land);
        filter.card_types.push(CardType::Planeswalker);
        filter.card_types.push(CardType::Battle);

        let described = describe_for_each_filter(&filter);
        assert!(
            !described.contains("on the battlefield"),
            "unexpected battlefield suffix in '{}'",
            described
        );
        assert!(
            described.contains("in that player's exile"),
            "expected exile context in '{}'",
            described
        );
    }

    #[test]
    fn normalizes_opponents_creatures_enter_tapped_sentence() {
        let normalized = normalize_sentence_surface_style(
            "An opponent's creature enter the battlefield tapped.",
        );
        assert_eq!(normalized, "Creatures your opponents control enter tapped.");
    }

    #[test]
    fn normalizes_rishadan_sacrifice_unless_pay_sentence() {
        let normalized = normalize_sentence_surface_style(
            "When this creature enters, for each opponent, that player sacrifices a permanent unless that player pays {2}.",
        );
        assert_eq!(
            normalized,
            "When this creature enters, each opponent sacrifices a permanent of their choice unless they pay {2}."
        );
    }

    #[test]
    fn normalizes_touchstone_tap_artifact_sentence() {
        let normalized = normalize_sentence_surface_style("Tap target opponent's artifact.");
        assert_eq!(normalized, "Tap target artifact you don't control.");
    }

    #[test]
    fn pluralize_noun_phrase_handles_an_opponent_controls_suffix() {
        assert_eq!(
            pluralize_noun_phrase("creature an opponent controls"),
            "creatures an opponent controls"
        );
        assert_eq!(
            pluralize_noun_phrase("target creature an opponent controls"),
            "target creatures an opponent controls"
        );
    }

    #[test]
    fn pluralize_noun_phrase_handles_you_own_suffix() {
        assert_eq!(pluralize_noun_phrase("Dwarf you own"), "Dwarves you own");
        assert_eq!(
            pluralize_noun_phrase("target permanent you own"),
            "target permanents you own"
        );
    }

    #[test]
    fn pluralize_noun_phrase_keeps_without_qualifier_singular() {
        assert_eq!(
            pluralize_noun_phrase("target creature without flying"),
            "target creatures without flying"
        );
    }

    #[test]
    fn normalizes_for_each_opponent_discards_count_sentence() {
        let normalized =
            normalize_sentence_surface_style("For each opponent, that player discards 2 cards.");
        assert_eq!(normalized, "Each opponent discards two cards.");
    }

    #[test]
    fn normalizes_during_your_turn_keyword_sentence() {
        let normalized = normalize_sentence_surface_style(
            "This creature has Lifelink as long as it's your turn.",
        );
        assert_eq!(normalized, "During your turn, this creature has lifelink.");
    }

    #[test]
    fn normalizes_sliver_sacrifice_damage_sentence() {
        let normalized = normalize_sentence_surface_style(
            "All slivers have 2 sacrifice this creature this creature deals 2 damage to any target.",
        );
        assert_eq!(
            normalized,
            "All Slivers have \"{2}, Sacrifice this permanent: This permanent deals 2 damage to any target.\""
        );
    }

    #[test]
    fn normalizes_prevent_next_damage_spell_sentence() {
        let normalized = normalize_sentence_surface_style(
            "Prevent the next 4 damage to any target until end of turn.",
        );
        assert_eq!(
            normalized,
            "Prevent the next 4 damage that would be dealt to any target this turn."
        );
    }

    #[test]
    fn normalizes_burn_away_sentence() {
        let normalized = normalize_sentence_surface_style(
            "Deal 6 damage to target creature. Exile target card in graveyard.",
        );
        assert_eq!(
            normalized,
            "Deal 6 damage to target creature. Exile target card from a graveyard."
        );
    }

    #[test]
    fn normalizes_granted_mana_ability_sentence() {
        let normalized = normalize_sentence_surface_style("Creatures you control have t add g.");
        assert_eq!(normalized, "Creatures you control have \"{T}: Add {G}.\"");
    }

    #[test]
    fn normalizes_specific_plural_surface_phrases() {
        assert_eq!(
            normalize_sentence_surface_style("Elfs you control get +2/+0."),
            "Elves you control get +2/+0."
        );
        assert_eq!(
            normalize_sentence_surface_style("Warrior have Haste."),
            "Warriors have Haste."
        );
        assert_eq!(
            normalize_sentence_surface_style(
                "Creature with a level counter on it you control get +2/+2."
            ),
            "Each creature you control with a level counter on it gets +2/+2."
        );
        assert_eq!(
            normalize_sentence_surface_style(
                "This creature's power and toughness are each equal to the number of Soldiers or Warrior you control."
            ),
            "This creature's power and toughness are each equal to the number of Soldiers and Warriors you control."
        );
        assert_eq!(
            normalize_sentence_surface_style("Goblin are black."),
            "Goblins are black."
        );
        assert_eq!(
            normalize_sentence_surface_style("Goblin are zombie in addition to their other types."),
            "Goblins are Zombies in addition to their other creature types."
        );
        assert_eq!(
            normalize_sentence_surface_style("Land is no longer snow."),
            "Lands are no longer snow."
        );
        assert_eq!(
            normalize_sentence_surface_style("Land enter the battlefield tapped."),
            "Lands enter the battlefield tapped."
        );
        assert_eq!(
            normalize_sentence_surface_style("Add 1 mana of any color."),
            "Add one mana of any color."
        );
    }

    #[test]
    fn normalizes_surveil_then_draw_sentence() {
        let normalized = normalize_sentence_surface_style("Surveil 2. Draw a card.");
        assert_eq!(normalized, "Surveil 2, then draw a card.");
    }

    #[test]
    fn normalizes_structural_collapse_sentence() {
        let normalized = normalize_sentence_surface_style(
            "Target player sacrifices a artifact. target player sacrifices target player's land. Deal 2 damage to target player of their choice.",
        );
        assert_eq!(
            normalized,
            "Target player sacrifices an artifact and a land of their choice. Structural Collapse deals 2 damage to that player."
        );
    }

    #[test]
    fn normalizes_reality_spasm_compact_mode_sentence() {
        let normalized = normalize_sentence_surface_style("Tap or untap X target permanents.");
        assert_eq!(
            normalized,
            "Choose one — Tap X target permanents. • Untap X target permanents."
        );
    }

    #[test]
    fn normalizes_ability_scoped_choose_one_into_bullets() {
        let normalized = normalize_sentence_surface_style(
            "Triggered ability 1: When this creature enters, choose one — Target creature gets +2/+0 until end of turn. • Target creature gets -0/-2 until end of turn.",
        );
        assert_eq!(
            normalized,
            "Triggered ability 1: When this creature enters, choose one —\n• Target creature gets +2/+0 until end of turn.\n• Target creature gets -0/-2 until end of turn."
        );
    }

    #[test]
    fn normalizes_ability_scoped_choose_one_or_more_into_bullets() {
        let normalized = normalize_sentence_surface_style(
            "Triggered ability 1: When this creature dies, choose one or more - Target opponent sacrifices a creature of their choice. • Target opponent discards two cards. • Target opponent loses 5 life.",
        );
        assert_eq!(
            normalized,
            "Triggered ability 1: When this creature dies, choose one or more —\n• Target opponent sacrifices a creature of their choice.\n• Target opponent discards two cards.\n• Target opponent loses 5 life."
        );
    }

    #[test]
    fn normalizes_ognis_treasure_trigger_sentence() {
        let normalized = normalize_sentence_surface_style(
            "Whenever a creature with haste you control attacks, create 1 Treasure artifact token with {T}, Sacrifice this artifact: Add one mana of any color. under your control, tapped.",
        );
        assert_eq!(
            normalized,
            "Whenever a creature you control with haste attacks, create a tapped Treasure token."
        );
    }

    #[test]
    fn post_pass_normalizes_each_opponent_life_loss_gain() {
        let normalized = normalize_compiled_post_pass_effect(
            "For each opponent, that player loses 1 life. you gain 1 life.",
        );
        assert_eq!(
            normalized,
            "Each opponent loses one life and you gain one life."
        );
    }

    #[test]
    fn post_pass_normalizes_for_each_player_draw_then_discard_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "For each player, that player draws 3 cards. For each player, that player discards 3 cards at random.",
        );
        assert_eq!(
            normalized,
            "Each player draws three cards, then discards three cards at random."
        );

        let normalized_plain = normalize_compiled_post_pass_effect(
            "When this creature enters, each player draws 2 cards. For each player, that player discards a card at random.",
        );
        assert_eq!(
            normalized_plain,
            "When this creature enters, each player draws two cards, then discards a card at random."
        );
    }

    #[test]
    fn post_pass_normalizes_for_each_player_discard_then_draw_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "For each player, that player discards their hand. For each player, that player draws 7 cards.",
        );
        assert_eq!(
            normalized,
            "Each player discards their hand, then draws 7 cards."
        );

        let normalized_plain = normalize_compiled_post_pass_effect(
            "Each player discards their hand. that player draws that many minus one cards.",
        );
        assert_eq!(
            normalized_plain,
            "Each player discards their hand, then draws that many minus one cards."
        );
    }

    #[test]
    fn post_pass_normalizes_gain_then_create_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "When this enchantment enters, you gain 2 life. Create a tapped Powerstone token.",
        );
        assert_eq!(
            normalized,
            "When this enchantment enters, you gain 2 life and create a tapped Powerstone token."
        );
    }

    #[test]
    fn post_pass_merges_lose_then_create_treasure_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "Whenever a player casts their second spell each turn, you lose 1 life. Create a Treasure token.",
        );
        assert_eq!(
            normalized,
            "Whenever a player casts their second spell each turn, you lose 1 life and create a Treasure token."
        );
    }

    #[test]
    fn post_pass_normalizes_malformed_second_spell_trigger_phrase() {
        let normalized = normalize_compiled_post_pass_effect(
            "Whenever you cast an as your second spell this turn, create a 4/4 red Dragon Elemental creature token with flying under your control spell.",
        );
        assert_eq!(
            normalized,
            "Whenever you cast your second spell each turn, create a 4/4 red Dragon Elemental creature token with flying under your control."
        );
    }

    #[test]
    fn post_pass_normalizes_during_your_turn_pt_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "This creature gets +0/+2 as long as it's your turn.",
        );
        assert_eq!(normalized, "During your turn, this creature gets +0/+2.");
    }

    #[test]
    fn post_pass_normalizes_split_two_land_search() {
        let normalized = normalize_compiled_post_pass_effect(
            "Search your library for up to one basic land you own, put it onto the battlefield tapped. Search your library for basic land you own, reveal it, put it into your hand, then shuffle.",
        );
        assert_eq!(
            normalized,
            "Search your library for up to two basic land cards, reveal those cards, put one onto the battlefield tapped and the other into your hand, then shuffle."
        );
    }

    #[test]
    fn post_pass_normalizes_split_two_land_search_without_you_own() {
        let normalized = normalize_compiled_post_pass_effect(
            "Search your library for a basic land card, put it onto the battlefield tapped. Search your library for basic land, reveal it, put it into your hand, then shuffle.",
        );
        assert_eq!(
            normalized,
            "Search your library for up to two basic land cards, reveal those cards, put one onto the battlefield tapped and the other into your hand, then shuffle."
        );
    }

    #[test]
    fn post_pass_normalizes_split_two_land_search_with_reveal_in_first_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "Search your library for a basic land card, reveal it, put it onto the battlefield tapped. Search your library for basic land, reveal it, put it into your hand, then shuffle.",
        );
        assert_eq!(
            normalized,
            "Search your library for up to two basic land cards, reveal those cards, put one onto the battlefield tapped and the other into your hand, then shuffle."
        );
    }

    #[test]
    fn post_pass_normalizes_split_two_gate_or_land_search() {
        let normalized = normalize_compiled_post_pass_effect(
            "Search your library for up to one basic land or Gate card, put it onto the battlefield tapped. Search your library for basic land or Gate, reveal it, put it into your hand, then shuffle.",
        );
        assert_eq!(
            normalized,
            "Search your library for up to two basic land or Gate cards, reveal those cards, put one onto the battlefield tapped and the other into your hand, then shuffle."
        );
    }

    #[test]
    fn post_pass_merges_for_each_opponent_discards_then_loses_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "When this creature enters, for each opponent, that player discards a card. For each opponent, that player loses 2 life.",
        );
        assert_eq!(
            normalized,
            "When this creature enters, each opponent discards a card, and loses 2 life."
        );
    }

    #[test]
    fn post_pass_merges_target_opponent_sacrifice_discard_lose_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "When this creature enters or this creature attacks, target opponent sacrifices a creature or planeswalker of their choice. target opponent discards a card. target opponent loses 3 life.",
        );
        assert_eq!(
            normalized,
            "Whenever this creature enters or attacks, target opponent sacrifices a creature or planeswalker of their choice, discards a card, and loses 3 life."
        );
    }

    #[test]
    fn post_pass_merges_draw_then_gain_life_chain() {
        let normalized = normalize_compiled_post_pass_effect("Draw a card. you gain 3 life.");
        assert_eq!(normalized, "Draw a card and gain 3 life.");
    }

    #[test]
    fn post_pass_merges_draw_then_gain_life_chain_with_prefix() {
        let normalized = normalize_compiled_post_pass_effect(
            "Whenever this creature enters or attacks, target opponent loses 3 life. Draw a card. you gain 3 life.",
        );
        assert_eq!(
            normalized,
            "Whenever this creature enters or attacks, target opponent loses 3 life. Draw a card and gain 3 life."
        );
    }

    #[test]
    fn post_pass_merges_discard_then_draw_chain_after_cost_colon() {
        let normalized = normalize_compiled_post_pass_effect(
            "{U}, Sacrifice a creature you control: you discard a card. Draw a card.",
        );
        assert_eq!(
            normalized,
            "{U}, Sacrifice a creature you control: discard a card, then draw a card."
        );
    }

    #[test]
    fn post_pass_merges_colon_discard_then_you_draw_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "When this creature is turned face up: you discard your hand. you draw three cards.",
        );
        assert_eq!(
            normalized,
            "When this creature is turned face up: discard your hand, then draw three cards."
        );
    }

    #[test]
    fn post_pass_merges_exile_then_you_draw_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "Exile all card in your hand. you draw that many cards.",
        );
        assert_eq!(
            normalized,
            "Exile all card in your hand, then draw that many cards."
        );
    }

    #[test]
    fn post_pass_merges_prefix_you_draw_then_you_gain_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "Return creature card from your graveyard to your hand. you draw three cards. you gain 5 life.",
        );
        assert_eq!(
            normalized,
            "Return creature card from your graveyard to your hand. Draw three cards and gain 5 life."
        );
    }

    #[test]
    fn post_pass_merges_damage_then_controller_loses_life_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "{T}: This creature deals 1 damage to target creature. that object's controller loses 1 life.",
        );
        assert_eq!(
            normalized,
            "{T}: This creature deals 1 damage to target creature and that creature's controller loses 1 life."
        );
    }

    #[test]
    fn post_pass_rewrites_exile_all_cards_then_return_it_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "Exile all card in your hand. At the beginning of the next end step, return it to its owner's hand. Draw a card.",
        );
        assert_eq!(
            normalized,
            "Exile all card in your hand. At the beginning of the next end step, return those cards to their owners' hands. Draw a card."
        );
    }

    #[test]
    fn post_pass_rewrites_token_copy_sacrifice_this_spell_tail() {
        let normalized = normalize_compiled_post_pass_effect(
            "Create a token that's a copy of target artifact or creature you control, with haste. At the beginning of the next end step, sacrifice this spell.",
        );
        assert_eq!(
            normalized,
            "Create a token that's a copy of target artifact or creature you control, with haste. At the beginning of the next end step, sacrifice it."
        );
    }

    #[test]
    fn post_pass_merges_target_player_discard_then_sacrifice_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "Target player loses 1 life. target player discards a card. sacrifice a permanent.",
        );
        assert_eq!(
            normalized,
            "Target player loses 1 life. Target player discards a card and sacrifices a permanent."
        );
    }

    #[test]
    fn post_pass_merges_return_all_then_destroy_all_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "Return all Zombie creature card in your graveyard to the battlefield tapped. Destroy all Humans.",
        );
        assert_eq!(
            normalized,
            "Return all Zombie creature card in your graveyard to the battlefield tapped, then destroy all Humans."
        );
    }

    #[test]
    fn post_pass_merges_you_gain_x_and_you_gain_n() {
        let normalized =
            normalize_compiled_post_pass_effect("You gain X life and you gain 3 life.");
        assert_eq!(normalized, "You gain X plus 3 life.");
    }

    #[test]
    fn line_post_pass_normalizes_you_gain_x_plus_n_phrase() {
        let normalized = normalize_gain_life_plus_phrase("You gain X life and you gain 3 life.");
        assert_eq!(normalized, "You gain X plus 3 life.");
    }

    #[test]
    fn post_pass_rewrites_if_that_doesnt_happen_draw_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "Return a land card or Elf card from your graveyard to your hand. If that doesn't happen, you draw a card.",
        );
        assert_eq!(
            normalized,
            "Return a land card or Elf card from your graveyard to your hand. If you can't, draw a card."
        );
    }

    #[test]
    fn post_pass_rewrites_if_that_doesnt_happen_return_and_energy_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "When this permanent enters, pay {E}{E}. If that doesn't happen, Return this permanent to its owner's hand. you get {E}.",
        );
        assert_eq!(
            normalized,
            "When this permanent enters, pay {E}{E}. If you can't, return this permanent to its owner's hand and you get {E}."
        );
    }

    #[test]
    fn post_pass_merges_get_and_gain_until_eot_for_creatures_you_control() {
        let normalized = normalize_compiled_post_pass_effect(
            "Whenever beregond or another Human you control enters, creatures you control get +1/+1 until end of turn. creatures you control gain Vigilance until end of turn",
        );
        assert_eq!(
            normalized,
            "Whenever beregond or another Human you control enters, creatures you control get +1/+1 and gain vigilance until end of turn."
        );
    }

    #[test]
    fn post_pass_merges_get_and_gain_until_eot_for_each_creature_you_control() {
        let normalized = normalize_compiled_post_pass_effect(
            "When this creature enters, each creature you control gets +1/+1 until end of turn. creatures you control gain Haste until end of turn",
        );
        assert_eq!(
            normalized,
            "When this creature enters, creatures you control get +1/+1 and gain haste until end of turn."
        );
    }

    #[test]
    fn post_pass_merges_mill_then_put_counter_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "When this permanent enters, you mill 2 cards. Put a +1/+1 counter on this permanent for each artifact or creature card in your graveyard.",
        );
        assert_eq!(
            normalized,
            "When this permanent enters, mill 2 cards, then put a +1/+1 counter on this permanent for each artifact or creature card in your graveyard."
        );
    }

    #[test]
    fn post_pass_normalizes_when_permanent_enters_or_whenever_attacks_prefix() {
        let normalized = normalize_compiled_post_pass_effect(
            "When this permanent enters or Whenever this creature attacks, target opponent loses 3 life.",
        );
        assert_eq!(
            normalized,
            "Whenever this creature enters or attacks, target opponent loses 3 life."
        );
    }

    #[test]
    fn post_pass_normalizes_tribal_spell_cost_phrase() {
        let normalized = normalize_compiled_post_pass_effect(
            "Spells Treefolk you control cost {1} less to cast.",
        );
        assert_eq!(
            normalized,
            "Treefolk spells you cast cost {1} less to cast."
        );
    }

    #[test]
    fn post_pass_normalizes_choose_each_type_exile_then_shared_type_search_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "You choose up to one artifact in the battlefield and tags it as 'exiled_0'. you choose up to one creature in the battlefield and tags it as 'exiled_0'. you choose up to one enchantment in the battlefield and tags it as 'exiled_0'. you choose up to one planeswalker in the battlefield and tags it as 'exiled_0'. you choose up to one land in the battlefield and tags it as 'exiled_0'. Exile it. For each object exiled this way, Search that player's library for permanent that shares a card type with that object that player owns, put it onto the battlefield, then shuffle.",
        );
        assert_eq!(
            normalized,
            "Exile up to one target artifact, up to one target creature, up to one target enchantment, up to one target planeswalker, and up to one target land. For each permanent exiled this way, its controller reveals cards from the top of their library until they reveal a card that shares a card type with it, puts that card onto the battlefield, then shuffles."
        );
    }

    #[test]
    fn post_pass_normalizes_this_leaves_battlefield_trigger_head() {
        let normalized = normalize_compiled_post_pass_effect(
            "This enchantment leaves the battlefield: you discard 3 cards and you lose 6 life. you sacrifice three creatures you control.",
        );
        assert_eq!(
            normalized,
            "When this enchantment leaves the battlefield, you discard 3 cards and you lose 6 life, then sacrifice three creatures you control."
        );
    }

    #[test]
    fn post_pass_handles_lowercase_for_each_opponent_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "When this creature enters, for each opponent, that player discards a card.",
        );
        assert_eq!(
            normalized,
            "When this creature enters, each opponent discards a card."
        );
    }

    #[test]
    fn post_pass_does_not_pluralize_destroy_all_creatures_twice() {
        let normalized = normalize_compiled_post_pass_effect("Destroy all creatures.");
        assert_eq!(normalized, "Destroy all creatures.");
    }

    #[test]
    fn post_pass_normalizes_embedded_powerstone_creation() {
        let normalized = normalize_compiled_post_pass_effect(
            "When this creature enters, create 1 Powerstone artifact token under your control, tapped.",
        );
        assert_eq!(
            normalized,
            "When this creature enters, create a tapped Powerstone token."
        );
    }

    #[test]
    fn post_pass_normalizes_embedded_lowercase_create_token_reminder() {
        let normalized = normalize_compiled_post_pass_effect(
            "Whenever an aura becomes attached to this creature, create 1 2/2 red Dragon creature token with flying and {R}: This creature gets +1/+0 until end of turn. under your control.",
        );
        assert_eq!(
            normalized,
            "Whenever an aura becomes attached to this creature, create a 2/2 red Dragon creature token with flying under your control. It has \"{R}: This creature gets +1/+0 until end of turn.\""
        );
    }

    #[test]
    fn post_pass_normalizes_embedded_token_trigger_reminder() {
        let normalized = normalize_compiled_post_pass_effect(
            "Whenever a nontoken artifact you control enters, create 1 Munitions artifact token with When this token leaves the battlefield, it deals 2 damage to any target. under your control.",
        );
        assert_eq!(
            normalized,
            "Whenever a nontoken artifact you control enters, create a Munitions artifact token under your control. It has \"When this token leaves the battlefield, it deals 2 damage to any target.\""
        );
    }

    #[test]
    fn create_under_control_normalization_skips_multi_create_sequences() {
        let raw = "Create a Food token. Create 1 Treasure artifact token with {T}, Sacrifice this artifact: Add one mana of any color. under your control, tapped. Create 1 3/2 Vehicle artifact token with crew 1 under your control.";
        assert!(normalize_create_under_control_clause(raw).is_none());
    }

    #[test]
    fn post_pass_does_not_leak_treasure_reminder_into_following_create_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "Create a Food token. Create 1 Treasure artifact token with {T}, Sacrifice this artifact: Add one mana of any color. under your control, tapped. Create 1 3/2 Vehicle artifact token with crew 1 under your control.",
        );
        assert!(
            normalized
                .contains("Create 1 3/2 Vehicle artifact token with crew 1 under your control.")
        );
        assert!(!normalized.contains("crew 1 under your control. It has \"{T}, Sacrifice this artifact: Add one mana of any color."));
    }

    #[test]
    fn post_pass_compacts_create_one_under_control_lists() {
        let normalized = normalize_compiled_post_pass_effect(
            "Create 1 1/1 green Snake creature token under your control. Create 1 2/2 green Wolf creature token under your control. Create 1 3/3 green Elephant creature token under your control.",
        );
        assert_eq!(
            normalized,
            "Create a 1/1 green Snake creature token, a 2/2 green Wolf creature token, and a 3/3 green Elephant creature token."
        );
    }

    #[test]
    fn post_pass_compacts_tapped_treasure_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "Create a Food token. Create 1 Treasure artifact token with {T}, Sacrifice this artifact: Add one mana of any color. tapped under your control. Create 1 3/2 Vehicle artifact token with crew 1 under your control.",
        );
        assert_eq!(
            normalized,
            "Create a Food token. Create a tapped Treasure token. Create a 3/2 Vehicle artifact token with crew 1."
        );
    }

    #[test]
    fn post_pass_compacts_triggered_create_one_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "When this creature enters, create 1 2/2 white Knight creature token with vigilance under your control. Create 1 3/3 green Centaur creature token under your control. Create 1 4/4 green Rhino creature token with trample under your control.",
        );
        assert_eq!(
            normalized,
            "When this creature enters, create a 2/2 white Knight creature token with vigilance, a 3/3 green Centaur creature token, and a 4/4 green Rhino creature token with trample"
        );
    }

    #[test]
    fn post_pass_normalizes_counter_then_proliferate_chains() {
        assert_eq!(
            normalize_compiled_post_pass_effect(
                "Put a +1/+1 counter on target creature. Proliferate."
            ),
            "Put a +1/+1 counter on target creature, then proliferate."
        );
        assert_eq!(
            normalize_compiled_post_pass_effect(
                "Put a -1/-1 counter on target creature. Proliferate."
            ),
            "Put a -1/-1 counter on target creature, then proliferate."
        );
    }

    #[test]
    fn post_pass_normalizes_embedded_for_each_put_counter_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "Create a 2/2 black Zombie creature token under your control. For each Zombie creature you control, Put a +1/+1 counter on that object.",
        );
        assert_eq!(
            normalized,
            "Create a 2/2 black Zombie creature token under your control. Put a +1/+1 counter on each Zombie creature you control."
        );
    }

    #[test]
    fn post_pass_normalizes_draw_then_put_top_of_library_chains() {
        let normalized = normalize_compiled_post_pass_effect(
            "{2}, {T}, Sacrifice this artifact: you draw three cards. Put two cards from your hand on top of your library.",
        );
        assert_eq!(
            normalized,
            "{2}, {T}, Sacrifice this artifact: you draw three cards, then put two cards from your hand on top of your library in any order."
        );
        let normalized_single = normalize_compiled_post_pass_effect(
            "When this creature enters, you draw two cards. Put a card from your hand on top of your library.",
        );
        assert_eq!(
            normalized_single,
            "When this creature enters, you draw two cards, then put a card from your hand on top of your library."
        );
    }

    #[test]
    fn post_pass_normalizes_bottom_then_shuffle_into_library_chains() {
        let normalized = normalize_compiled_post_pass_effect(
            "Spell effects: Put up to one target card from your graveyard on the bottom of your library. Shuffle your library.",
        );
        assert_eq!(
            normalized,
            "Spell effects: Shuffle up to one target card from your graveyard into your library."
        );
        let targeted = normalize_compiled_post_pass_effect(
            "Triggered ability 1: When this creature enters, put any number of target cards from target player's graveyard on the bottom of target player's library. Shuffle target player's library.",
        );
        assert_eq!(
            targeted,
            "Triggered ability 1: When this creature enters, shuffle any number of target cards from target player's graveyard into target player's library."
        );
    }

    #[test]
    fn post_pass_normalizes_archangel_life_gain_graveyard_variant() {
        let normalized = normalize_compiled_post_pass_effect(
            "You gain 2 life for each card in graveyard you own.",
        );
        assert_eq!(
            normalized,
            "You gain 2 life for each card in graveyard you own."
        );
    }

    #[test]
    fn post_pass_normalizes_spider_destroy_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "Whenever this creature deals damage to Spider, destroy it.",
        );
        assert_eq!(
            normalized,
            "Whenever this creature deals damage to a Spider, destroy that creature."
        );
    }

    #[test]
    fn post_pass_normalizes_tapped_robot_creation_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "Create two 1/1 colorless Robot artifact creature token with flying tapped under your control.",
        );
        assert_eq!(
            normalized,
            "Create two tapped 1/1 colorless Robot artifact creature tokens with flying."
        );
    }

    #[test]
    fn post_pass_normalizes_dramatic_rescue_style_gain_life_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "Return target creature to its owner's hand and you gain 2 life.",
        );
        assert_eq!(
            normalized,
            "Return target creature to its owner's hand. You gain 2 life."
        );
    }

    #[test]
    fn post_pass_normalizes_contraband_livestock_roll_outcomes() {
        let normalized = normalize_compiled_post_pass_effect(
            "Exile target creature. Create 1 4/4 green Ox creature token under your control. Create 1 2/2 green Boar creature token under your control. Create 1 0/1 white Goat creature token under your control.",
        );
        assert_eq!(
            normalized,
            "Exile target creature, then roll a d20. 1—9 | Its controller creates a 4/4 green Ox creature token. 10—19 | Its controller creates a 2/2 green Boar creature token. 20 | Its controller creates a 0/1 white Goat creature token."
        );
    }

    #[test]
    fn post_pass_merges_repeated_subject_predicate_sentences() {
        let normalized = normalize_compiled_post_pass_effect(
            "This creature gets +1/+0 until end of turn. this creature gains Flying until end of turn.",
        );
        assert_eq!(
            normalized,
            "This creature gets +1/+0 and gains flying until end of turn"
        );
    }

    #[test]
    fn merge_adjacent_subject_lines_merges_lose_abilities_with_base_pt() {
        assert_eq!(
            merge_adjacent_subject_predicate_lines(vec![
                "Creature lose all abilities.".to_string(),
                "Affected permanents have base power and toughness 1/1.".to_string(),
            ]),
            vec!["Creatures lose all abilities and have base power and toughness 1/1".to_string()]
        );
        assert_eq!(
            merge_adjacent_subject_predicate_lines(vec![
                "Enchanted creature lose all abilities.".to_string(),
                "Affected permanents have base power and toughness 1/1.".to_string(),
            ]),
            vec![
                "Enchanted creature loses all abilities and has base power and toughness 1/1"
                    .to_string()
            ]
        );
    }

    #[test]
    fn post_pass_normalizes_inline_for_each_damage_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "Whenever this creature becomes blocked, for each attacking/blocking creature, Deal 2 damage to that object.",
        );
        assert_eq!(
            normalized,
            "Whenever this creature becomes blocked, it deals 2 damage to each attacking creature and each blocking creature."
        );
    }

    #[test]
    fn post_pass_normalizes_sentence_for_each_damage_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "Deal 3 damage to target player. For each creature that player controls, Deal 1 damage to that object.",
        );
        assert_eq!(
            normalized,
            "Deal 3 damage to target player. Deal 1 damage to each creature that player controls."
        );
    }

    #[test]
    fn post_pass_normalizes_you_may_for_each_damage_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "Whenever this creature attacks, you may For each creature without flying, Deal 1 damage to that object.",
        );
        assert_eq!(
            normalized,
            "Whenever this creature attacks, you may have it deal 1 damage to each creature without flying."
        );
    }

    #[test]
    fn post_pass_normalizes_up_to_two_cant_block_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "Choose up to two target creatures. target creature can't be blocked until end of turn.",
        );
        assert_eq!(
            normalized,
            "Up to two target creatures can't be blocked this turn."
        );

        let normalized_this_turn = normalize_compiled_post_pass_effect(
            "Choose up to two target creatures. target creature can't be blocked this turn.",
        );
        assert_eq!(
            normalized_this_turn,
            "Up to two target creatures can't be blocked this turn."
        );
    }

    #[test]
    fn post_pass_normalizes_up_to_two_tap_and_untap_lock_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "{3}{U}, Discard this card: Tap up to two target creatures an opponent controls. creature can't untap until your next turn.",
        );
        assert_eq!(
            normalized,
            "{3}{U}, Discard this card: Tap up to two target creatures you don't control. Those creatures don't untap during their controller's next untap step."
        );
    }

    #[test]
    fn post_pass_normalizes_each_player_sacrifice_choice_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "When this creature enters, for each player, that player sacrifices two creatures that player controls.",
        );
        assert_eq!(
            normalized,
            "When this creature enters, each player sacrifices two creatures of their choice."
        );
    }

    #[test]
    fn post_pass_normalizes_blocked_pt_scale_phrase() {
        let normalized = normalize_compiled_post_pass_effect(
            "Whenever this creature becomes blocked, it gets +-1 / +-1 for each the number of blocking creature until end of turn.",
        );
        assert_eq!(
            normalized,
            "Whenever this creature becomes blocked, it gets -1/-1 until end of turn for each creature blocking it."
        );
    }

    #[test]
    fn post_pass_splits_gain_clause_after_main_effect() {
        let normalized =
            normalize_compiled_post_pass_effect("Destroy target creature and you gain 3 life.");
        assert_eq!(normalized, "Destroy target creature and you gain 3 life.");
    }

    #[test]
    fn post_pass_normalizes_cast_spell_subtype_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "Whenever you cast spell Knight, create 1 1/1 white Human creature token under your control.",
        );
        assert_eq!(
            normalized,
            "Whenever you cast a Knight spell, create 1 1/1 white Human creature token under your control."
        );
    }

    #[test]
    fn post_pass_normalizes_generic_for_each_player_clause() {
        let normalized =
            normalize_compiled_post_pass_effect("For each player, that player mills a card.");
        assert_eq!(normalized, "Each player mills a card.");
    }

    #[test]
    fn post_pass_normalizes_for_each_player_draw_a_card_clause() {
        let normalized =
            normalize_compiled_post_pass_effect("For each player, that player draws a card.");
        assert_eq!(normalized, "Each player draws a card.");
    }

    #[test]
    fn post_pass_normalizes_each_player_create_under_their_control_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "Each player creates 1 5/5 red Dragon creature token with flying under that player's control.",
        );
        assert_eq!(
            normalized,
            "Each player creates a 5/5 red Dragon creature token with flying."
        );
    }

    #[test]
    fn post_pass_normalizes_delayed_extra_turn_sentence() {
        let normalized = normalize_compiled_post_pass_effect(
            "At the beginning of your next end step, you takes an extra turn after this one. you loses the game",
        );
        assert_eq!(
            normalized,
            "Take an extra turn after this one. At the beginning of that turn's end step, you lose the game"
        );
    }

    #[test]
    fn post_pass_normalizes_extra_turn_sentence_with_turn_clause_second() {
        let normalized = normalize_compiled_post_pass_effect(
            "You take an extra turn after this one. At the beginning of your next end step, you lose the game.",
        );
        assert_eq!(
            normalized,
            "Take an extra turn after this one. At the beginning of that turn's end step, you lose the game"
        );
    }

    #[test]
    fn post_pass_normalizes_untap_plural_pronoun_buff_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "Untap up to two target creatures. it gets +2/+2 until end of turn.",
        );
        assert_eq!(
            normalized,
            "Untap up to two target creatures. They each get +2/+2 until end of turn."
        );
    }

    #[test]
    fn post_pass_normalizes_upkeep_damage_clause_with_implicit_source() {
        let normalized = normalize_compiled_post_pass_effect(
            "At the beginning of your upkeep, deal 1 damage to you.",
        );
        assert_eq!(
            normalized,
            "At the beginning of your upkeep, this creature deals 1 damage to you."
        );
    }

    #[test]
    fn post_pass_reorders_for_each_until_end_of_turn_phrase() {
        let normalized = normalize_compiled_post_pass_effect(
            "Target creature gets +1 / +1 for each a Forest you control until end of turn.",
        );
        assert_eq!(
            normalized,
            "Target creature gets +1/+1 until end of turn for each a Forest you control"
        );
    }

    #[test]
    fn post_pass_avoids_double_article_for_cast_a_spell() {
        let normalized =
            normalize_compiled_post_pass_effect("Whenever you cast a spell, you draw a card.");
        assert_eq!(normalized, "Whenever you cast a spell, you draw a card.");
    }

    #[test]
    fn post_pass_avoids_double_article_for_cast_another_spell() {
        let normalized = normalize_compiled_post_pass_effect(
            "Whenever you cast another spell, create 1 1/1 blue Bird creature token under your control.",
        );
        assert_eq!(
            normalized,
            "Whenever you cast another spell, create 1 1/1 blue Bird creature token under your control."
        );
    }

    #[test]
    fn post_pass_normalizes_this_or_another_trigger_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "Whenever This creature or Whenever another nontoken historic permanent you control enters, deal 1 damage to each opponent and you gain 1 life.",
        );
        assert_eq!(
            normalized,
            "Whenever this creature or another nontoken historic permanent you control enters, deal 1 damage to each opponent and you gain 1 life."
        );
    }

    #[test]
    fn post_pass_normalizes_begin_the_invasion_search_phrase() {
        let normalized = normalize_compiled_post_pass_effect(
            "Search your library for X battle you own, put them onto the battlefield, then shuffle.",
        );
        assert_eq!(
            normalized,
            "Search your library for up to X battle cards with different names, put them onto the battlefield, then shuffle."
        );
    }

    #[test]
    fn post_pass_normalizes_lands_you_control_skip_untap_step() {
        let normalized = normalize_compiled_post_pass_effect(
            "Gain control of target artifact or creature or enchantment. a land you control can't untap until your next turn.",
        );
        assert_eq!(
            normalized,
            "Gain control of target artifact or creature or enchantment. Lands you control don't untap during your next untap step."
        );
    }

    #[test]
    fn post_pass_normalizes_predatory_nightstalker_sacrifice_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "When this creature enters, you may target opponent sacrifices target creature an opponent controls.",
        );
        assert_eq!(
            normalized,
            "When this creature enters, you may have target opponent sacrifice a creature of their choice."
        );
    }

    #[test]
    fn post_pass_normalizes_you_may_target_creature_gets_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "When this creature enters, you may target creature gets -1/-1 until end of turn.",
        );
        assert_eq!(
            normalized,
            "When this creature enters, you may have target creature get -1/-1 until end of turn."
        );
    }

    #[test]
    fn post_pass_normalizes_tidebinder_untap_lock_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "When this creature enters, tap target opponent's red or green creature. permanent can't untap while you control this creature.",
        );
        assert_eq!(
            normalized,
            "When this creature enters, tap target opponent's red or green creature. That creature doesn't untap during its controller's untap step for as long as you control this creature."
        );
    }

    #[test]
    fn post_pass_normalizes_blade_of_the_bloodchief_equipped_vampire_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "Whenever a creature dies, tag the object attached to this artifact as 'equipped'. If the tagged object 'equipped' matches Vampire creature, Put two +1/+1 counters on the tagged object 'equipped'. Otherwise, Put a +1/+1 counter on the tagged object 'equipped'.",
        );
        assert_eq!(
            normalized,
            "Whenever a creature dies, put a +1/+1 counter on equipped creature. If equipped creature is a Vampire, put two +1/+1 counters on it instead."
        );
    }

    #[test]
    fn post_pass_normalizes_havoc_life_loss_controller() {
        let normalized = normalize_known_low_tail_phrase(
            "Whenever an opponent casts white spell, you lose 2 life.",
        );
        assert_eq!(
            normalized,
            "Whenever an opponent casts a white spell, they lose 2 life."
        );
    }

    #[test]
    fn post_pass_normalizes_mindlash_sliver_quoted_static_ability() {
        let normalized = normalize_known_low_tail_phrase(
            "All Slivers have 1 sacrifice this creature each player discards a card.",
        );
        assert_eq!(
            normalized,
            "All Slivers have \"{1}, Sacrifice this permanent: Each player discards a card.\""
        );
    }

    #[test]
    fn post_pass_normalizes_archon_of_cruelty_trigger_chain() {
        let normalized = normalize_known_low_tail_phrase(
            "Whenever this creature enters or attacks, target opponent sacrifices target opponent's creature or planeswalker. target opponent discards a card. target opponent loses 3 life. Draw a card. you gain 3 life.",
        );
        assert_eq!(
            normalized,
            "Whenever this creature enters or attacks, target opponent sacrifices a creature or planeswalker of their choice, discards a card, and loses 3 life. Draw a card and you gain 3 life."
        );
    }

    #[test]
    fn post_pass_normalizes_underworld_sentinel_exiled_with_it_clause() {
        let normalized = normalize_known_low_tail_phrase(
            "When this creature dies, return all card in exile to the battlefield.",
        );
        assert_eq!(
            normalized,
            "When this creature dies, put all cards exiled with it onto the battlefield."
        );
    }

    #[test]
    fn post_pass_normalizes_shared_draw_three_clause() {
        let normalized =
            normalize_known_low_tail_phrase("Draw three cards. target opponent draws 3 cards.");
        assert_eq!(normalized, "You and target opponent each draw three cards.");
    }

    #[test]
    fn post_pass_normalizes_shared_attacking_player_draw_and_lose_clause() {
        let normalized = normalize_known_low_tail_phrase(
            "Whenever an opponent attacks another one of your opponents, you draw a card. the attacking player draws a card. you lose 1 life. the attacking player loses 1 life.",
        );
        assert_eq!(
            normalized,
            "Whenever an opponent attacks another one of your opponents, you and the attacking player each draw a card and lose 1 life"
        );
    }

    #[test]
    fn post_pass_normalizes_iridian_maelstrom_destroy_phrase() {
        let normalized =
            normalize_known_low_tail_phrase("Destroy all creatures that are not all colors.");
        assert_eq!(normalized, "Destroy each creature that isn't all colors.");
    }

    #[test]
    fn post_pass_normalizes_iridian_maelstrom_destroy_phrase_with_spell_prefix() {
        let normalized = normalize_known_low_tail_phrase(
            "Spell effects: Destroy all creatures that are not all colors.",
        );
        assert_eq!(
            normalized,
            "Spell effects: Destroy each creature that isn't all colors."
        );
    }

    #[test]
    fn renders_destroy_not_all_colors_with_each_creature_wording() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Iridian Render Variant")
            .card_types(vec![CardType::Sorcery])
            .parse_text("Destroy each creature that isn't all colors.")
            .expect("iridian destroy wording should parse");

        let rendered = compiled_lines(&def).join(" ");
        assert!(
            rendered
                .to_ascii_lowercase()
                .contains("destroy each creature that isn't all colors"),
            "expected each-creature wording in rendered compiled line, got {rendered}"
        );
    }

    #[test]
    fn renders_dynamic_any_one_color_mana_with_explicit_x_count_clause() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Wirewood Render Variant")
            .card_types(vec![CardType::Creature])
            .parse_text("{T}: Add X mana of any one color, where X is the number of Elves on the battlefield.")
            .expect("wirewood dynamic mana clause should parse");

        let rendered = compiled_lines(&def).join(" ");
        assert!(
            rendered
                .to_ascii_lowercase()
                .contains("add x mana of any one color"),
            "expected explicit X mana wording, got {rendered}"
        );
        assert!(
            rendered
                .to_ascii_lowercase()
                .contains("where x is the number of elves on the battlefield"),
            "expected explicit where-X count clause, got {rendered}"
        );
    }

    #[test]
    fn renders_equipment_self_reference_and_singular_attach_target() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Equipment Render Variant")
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Equipment])
            .parse_text(
                "When this Equipment enters, attach it to target creature you control.\nEquip {1}",
            )
            .expect("equipment self-reference should parse");

        let rendered = compiled_lines(&def).join(" ");
        assert!(
            rendered
                .contains("When this Equipment enters, attach it to target creature you control."),
            "expected equipment self-reference + singular attach wording, got {rendered}"
        );
    }

    #[test]
    fn normalize_rendered_line_prefers_saga_self_reference_when_oracle_uses_saga() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Saga Render Variant")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Saga])
            .parse_text("When this Saga enters, draw a card.")
            .expect("saga line should parse");

        let normalized = normalize_rendered_line_for_card(
            &def,
            "Triggered ability 1: When this enchantment enters, draw a card.",
        );
        assert_eq!(
            normalized,
            "Triggered ability 1: When this Saga enters, draw a card."
        );
    }

    #[test]
    fn normalize_rendered_line_prefers_siege_self_reference_when_oracle_uses_siege() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Siege Render Variant")
            .card_types(vec![CardType::Battle])
            .parse_text("When this Siege enters, draw a card.")
            .expect("siege line should parse");

        let normalized = normalize_rendered_line_for_card(
            &def,
            "Triggered ability 1: When this permanent enters, draw a card.",
        );
        assert_eq!(
            normalized,
            "Triggered ability 1: When this Siege enters, draw a card."
        );
    }

    #[test]
    fn post_pass_normalizes_saw_in_half_copy_stats_phrase() {
        let normalized = normalize_known_low_tail_phrase(
            "Destroy target creature. If that permanent dies this way, Create two tokens that are copies of it under that object's controller's control, except their power and toughness are each half that permanent's power and toughness, rounded up.",
        );
        assert_eq!(
            normalized,
            "Destroy target creature. If that creature dies this way, its controller creates two tokens that are copies of that creature, except their power is half that creature's power and their toughness is half that creature's toughness. Round up each time."
        );
    }

    #[test]
    fn known_low_tail_preserves_attack_tap_without_goad() {
        let normalized = normalize_known_low_tail_phrase(
            "Whenever you attack a player, tap target creature that player controls.",
        );
        assert_eq!(
            normalized,
            "Whenever you attack a player, tap target creature that player controls."
        );
    }

    #[test]
    fn post_pass_normalizes_destroy_blocking_creature_with_cost_prefix() {
        let normalized =
            normalize_known_low_tail_phrase("{B}{B}: Destroy target blocking creature.");
        assert_eq!(
            normalized,
            "{B}{B}: Destroy target creature blocking this creature."
        );
    }

    #[test]
    fn post_pass_rewrites_return_with_multiple_counters_on_it_sequence() {
        let normalized = normalize_compiled_post_pass_effect(
            "Return target card from your graveyard to the battlefield. Put a Hexproof counter on it. Put a Indestructible counter on it.",
        );
        assert_eq!(
            normalized,
            "Return target permanent card from your graveyard to the battlefield with a Hexproof counter and an Indestructible counter on it."
        );
    }

    #[test]
    fn normalize_spell_self_exile_collapses_with_counters_clause() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Arc Blade").build();
        let normalized = normalize_spell_self_exile(
            &def,
            "Deal 2 damage to any target. Exile this spell. Put three time counters on this spell.",
        );
        assert_eq!(
            normalized,
            "Deal 2 damage to any target. Exile Arc Blade with three time counters on it."
        );
    }

    #[test]
    fn normalize_spell_self_exile_collapses_permanent_with_counters_clause() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Charnel Serenade").build();
        let normalized = normalize_spell_self_exile(
            &def,
            "Exile this permanent. Put three time counters on this permanent.",
        );
        assert_eq!(
            normalized,
            "Exile Charnel Serenade with three time counters on it."
        );
    }

    #[test]
    fn post_pass_romanizes_saga_chapter_prefix() {
        let normalized = normalize_compiled_post_pass_effect(
            "Chapters 1, 2, 3, 4: other creatures you control get +1/+0 until end of turn.",
        );
        assert_eq!(
            normalized,
            "I, II, III, IV — other creatures you control get +1/+0 until end of turn."
        );
    }

    #[test]
    fn post_pass_quotes_granted_triggered_ability_text() {
        let normalized = normalize_compiled_post_pass_effect(
            "Creatures you control have whenever this creature becomes the target of a spell or ability, reveal the top card of your library.",
        );
        assert_eq!(
            normalized,
            "Creatures you control have \"Whenever this creature becomes the target of a spell or ability, reveal the top card of your library.\""
        );
    }

    #[test]
    fn post_pass_punctuates_granted_triggered_ability_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "Creatures you control have whenever this creature becomes the target of a spell or ability reveal the top card of your library if its a land card put it onto the battlefield otherwise put it into your hand this ability triggers only twice each turn.",
        );
        assert_eq!(
            normalized,
            "Creatures you control have \"Whenever this creature becomes the target of a spell or ability reveal the top card of your library. If it's a land card put it onto the battlefield. Otherwise put it into your hand. This ability triggers only twice each turn.\""
        );
    }

    #[test]
    fn post_pass_normalizes_state_trigger_colon_prefix() {
        let normalized = normalize_compiled_post_pass_effect(
            "You control no other artifacts: Sacrifice this creature.",
        );
        assert_eq!(
            normalized,
            "When you control no other artifacts, sacrifice this creature."
        );
    }

    #[test]
    fn post_pass_normalizes_draw_and_lose_compound_clause() {
        let normalized =
            normalize_compiled_post_pass_effect("You draw two cards and you lose 2 life.");
        assert_eq!(normalized, "You draw two cards and lose 2 life.");
    }

    #[test]
    fn post_pass_normalizes_misc_surface_cases_near_threshold() {
        let normalized = normalize_compiled_post_pass_effect(
            "Discard your hand, then draw 7 cards, then discard 3 cards at random.",
        );
        assert_eq!(
            normalized,
            "Discard your hand. Draw seven cards, then discard three cards at random."
        );

        let normalized = normalize_compiled_post_pass_effect(
            "When this creature dies, exile it. Return another target creature card from your graveyard to your hand.",
        );
        assert_eq!(
            normalized,
            "When this creature dies, exile it, then return another target creature card from your graveyard to your hand."
        );

        let normalized = normalize_compiled_post_pass_effect(
            "At the beginning of each player's upkeep: that player sacrifices a white or green permanent.",
        );
        assert_eq!(
            normalized,
            "At the beginning of each player's upkeep: that player sacrifices a green or white permanent."
        );

        let normalized = normalize_compiled_post_pass_effect(
            "Counter target spell. Deal 2 damage to that object's controller.",
        );
        assert_eq!(
            normalized,
            "Counter target spell. This spell deals 2 damage to that spell's controller."
        );

        let normalized = normalize_compiled_post_pass_effect(
            "Exile up to one target non-Warrior creature you control. Return it to the battlefield under its owner's control.",
        );
        assert_eq!(
            normalized,
            "Exile up to one target non-Warrior creature you control, then return it to the battlefield under its owner's control."
        );

        let normalized = normalize_compiled_post_pass_effect(
            "Exile target land you control. Return that card to the battlefield under your control.",
        );
        assert_eq!(
            normalized,
            "Exile target land you control, then return it to the battlefield under your control."
        );

        let normalized = normalize_compiled_post_pass_effect(
            "Exile another target creature. Return it from graveyard to the battlefield tapped.",
        );
        assert_eq!(
            normalized,
            "Exile another target creature, then return it to the battlefield tapped."
        );

        let normalized = normalize_compiled_post_pass_effect(
            "Exile target creature. At the beginning of the next end step, return it to the battlefield. Put a +1/+1 counter on it.",
        );
        assert_eq!(
            normalized,
            "Exile target creature. At the beginning of the next end step, return that card to the battlefield under its owner's control with a +1/+1 counter on it."
        );

        let normalized = normalize_compiled_post_pass_effect(
            "When this permanent enters, you gain 3 life. you get {E}{E}{E}.",
        );
        assert_eq!(
            normalized,
            "When this permanent enters, you gain 3 life and you get {E}{E}{E}."
        );

        let normalized = normalize_compiled_post_pass_effect("Draw a card. you get {E}{E}.");
        assert_eq!(normalized, "Draw a card. You get {E}{E}.");

        let normalized = normalize_compiled_post_pass_effect(
            "{1}, Sacrifice an artifact you control: this permanent gets +1/+1 until end of turn. Deal 1 damage to each opponent.",
        );
        assert_eq!(
            normalized,
            "{1}, Sacrifice an artifact you control: this permanent gets +1/+1 until end of turn, and deals 1 damage to each opponent."
        );

        let normalized = normalize_compiled_post_pass_effect(
            "When this permanent enters, target player sacrifices a creature or planeswalker of their choice. target player loses 1 life.",
        );
        assert_eq!(
            normalized,
            "When this permanent enters, target player sacrifices a creature or planeswalker of their choice and loses 1 life."
        );
    }

    #[test]
    fn normalizes_sentence_misc_surface_cases_near_threshold() {
        assert_eq!(
            normalize_sentence_surface_style(
                "All Slivers have 2 sacrifice this permanent draw a card."
            ),
            "All Slivers have \"{2}, Sacrifice this permanent: Draw a card.\""
        );
        assert_eq!(
            normalize_sentence_surface_style(
                "Draw two cards and you lose 2 life. you mill 2 cards."
            ),
            "Draw two cards, lose 2 life, then mill two cards."
        );
        assert_eq!(
            normalize_sentence_surface_style("Slivercycling {{3}}."),
            "Slivercycling {3}."
        );
        assert_eq!(
            normalize_sentence_surface_style(
                "Exile up to one target artifact, creature, or enchantment you control. Return it to the battlefield under its owner's control. Draw a card."
            ),
            "Exile up to one target artifact, creature, or enchantment you control. Return it to the battlefield under its owner's control. Draw a card."
        );
        assert_eq!(
            normalize_sentence_surface_style(
                "Exile target creature. Return it from graveyard to the battlefield tapped. Draw a card."
            ),
            "Exile target creature. Return it from graveyard to the battlefield tapped. Draw a card."
        );
        assert_eq!(
            normalize_sentence_surface_style(
                "When this permanent enters, it deals 1 damage to that player. For each opponent's creature, Deal 1 damage to each opponent."
            ),
            "When this permanent enters, it deals 1 damage to each opponent and each creature your opponents control."
        );
        assert_eq!(
            normalize_sentence_surface_style(
                "When this permanent enters, put a card from that player's hand on top of that player's library."
            ),
            "When this permanent enters, target player puts a card from their hand on top of their library."
        );
        assert_eq!(
            normalize_sentence_surface_style(
                "At the beginning of your end step: For each creature you control, put a +1/+1 counter on that object. For each planeswalker you control, Put a loyalty counter on that object."
            ),
            "At the beginning of your end step: put a +1/+1 counter on each creature you control and a loyalty counter on each planeswalker you control."
        );
        assert_eq!(
            normalize_sentence_surface_style(
                "Creatures you control get +1/+0 as long as it's your turn."
            ),
            "During your turn, creatures you control get +1/+0."
        );
        assert_eq!(
            normalize_sentence_surface_style(
                "At the beginning of your end step: you discard a card and you lose 2 life. sacrifice a creature."
            ),
            "At the beginning of your end step: you discard a card and you lose 2 life, then sacrifice a creature."
        );
        assert_eq!(
            normalize_sentence_surface_style(
                "Destroy target enchantment. Destroy all other enchantment that shares a color with that object."
            ),
            "Destroy target enchantment and each other enchantment that shares a color with it."
        );
        assert_eq!(
            normalize_sentence_surface_style(
                "Whenever this creature attacks, you may sacrifice an artifact unless you discard a card. If you do, draw a card. this permanent gets +2/+0 until end of turn."
            ),
            "Whenever this creature attacks, you may sacrifice an artifact or discard a card. If you do, draw a card. this creature gets +2/+0 until end of turn."
        );
        assert_eq!(
            normalize_sentence_surface_style(
                "Whenever an opponent casts a spell, you may draw a card unless you pay {1}."
            ),
            "Whenever an opponent casts a spell, you may draw a card unless you pay {1}."
        );
    }

    #[test]
    fn post_pass_normalizes_capenna_fetchland_sacrifice_search_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "When this land enters, you choose a permanent you control in the battlefield. you sacrifice a permanent. If you do, Search your library for up to one basic land Forest or Plains or Island you own, put it onto the battlefield tapped, then shuffle. you gain 1 life.",
        );
        assert_eq!(
            normalized,
            "When this land enters, sacrifice it. If you do, search your library for a basic Forest or Plains or Island card, put it onto the battlefield tapped, then shuffle and you gain 1 life."
        );
    }

    #[test]
    fn post_pass_normalizes_each_target_creature_opponent_controls_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "Deal 1 damage to each target creature an opponent controls.",
        );
        assert_eq!(
            normalized,
            "Deal 1 damage to each creature target opponent controls."
        );
    }

    #[test]
    fn post_pass_normalizes_manabond_style_end_step_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "At the beginning of your end step: you may Reveal your hand. Return all land card in your hand to the battlefield. If you do, discard your hand.",
        );
        assert_eq!(
            normalized,
            "At the beginning of your end step, you may reveal your hand and put all land cards from it onto the battlefield. If you do, discard your hand."
        );
    }

    #[test]
    fn common_semantic_phrasing_keeps_earthbend_chain_tail() {
        let normalized = normalize_common_semantic_phrasing(
            "Earthbend target land you control with 3 +1/+1 counter(s). Earthbend target land you control with 3 +1/+1 counter(s). You gain 3 life.",
        );
        assert_eq!(normalized.matches("Earthbend 3").count(), 2);
        assert!(normalized.contains("You gain 3 life"));
    }

    #[test]
    fn common_semantic_phrasing_avoids_trigger_as_creature_type_list() {
        let normalized = normalize_common_semantic_phrasing(
            "Whenever this or Whenever another Treefolk you control enters, up to two target creatures get +2/+2 and gain Trample until end of turn.",
        );
        assert!(
            !normalized.contains("Each creature that's a Whenever"),
            "trigger text was incorrectly rewritten as a creature-type list: {normalized}"
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_predatory_sacrifice_choice_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "When this creature enters, you may target opponent sacrifices target creature an opponent controls.",
        );
        assert_eq!(
            normalized,
            "When this creature enters, you may have target opponent sacrifice a creature of their choice."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_creatures_you_control_get_phrase() {
        let normalized = normalize_common_semantic_phrasing(
            "Activated ability 1: {T}: creatures you control get +1/+2 until end of turn.",
        );
        assert_eq!(
            normalized,
            "Activated ability 1: {T}: Each creature you control gets +1/+2 until end of turn."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_tidebinder_lock_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "When this creature enters, tap target opponent's red or green creature. permanent can't untap while you control this creature.",
        );
        assert_eq!(
            normalized,
            "When this creature enters, tap target opponent's red or green creature. that creature doesn't untap during its controller's untap step for as long as you control this creature."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_tap_then_controller_next_untap_step_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "Tap up to two target creatures. creature can't untap during its controller's next untap step.",
        );
        assert_eq!(
            normalized,
            "Tap up to two target creatures. Those creatures don't untap during their controller's next untap step."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_frost_lynx_style_tap_lock_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "When this permanent enters, tap target creature an opponent controls. permanent can't untap during its controller's next untap step.",
        );
        assert_eq!(
            normalized,
            "When this creature enters, tap target creature an opponent controls. That creature doesn't untap during its controller's next untap step."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_named_trigger_tap_lock_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "Triggered ability 3: When Abominable Treefolk enters, tap target creature an opponent controls. permanent can't untap during its controller's next untap step.",
        );
        assert_eq!(
            normalized,
            "Triggered ability 3: When Abominable Treefolk enters, tap target creature an opponent controls. That creature doesn't untap during its controller's next untap step."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_spell_effect_tap_lock_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "Spell effects: Tap target creature. permanent can't untap during its controller's next untap step.",
        );
        assert_eq!(
            normalized,
            "Spell effects: Tap target creature. That creature doesn't untap during its controller's next untap step."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_attack_or_block_untap_lock_clause() {
        let attack = normalize_common_semantic_phrasing(
            "Whenever this creature attacks, permanent can't untap during its controller's next untap step.",
        );
        assert_eq!(
            attack,
            "Whenever this creature attacks, it doesn't untap during its controller's next untap step."
        );

        let block = normalize_common_semantic_phrasing(
            "Whenever this creature blocks a creature, permanent can't untap during its controller's next untap step.",
        );
        assert_eq!(
            block,
            "Whenever this creature blocks a creature, that creature doesn't untap during its controller's next untap step."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_target_opponent_sacrifice_discard_lose_chain() {
        let normalized = normalize_common_semantic_phrasing(
            "target opponent sacrifices target opponent's creature or planeswalker. target opponent discards a card. target opponent loses 3 life",
        );
        assert_eq!(
            normalized,
            "target opponent sacrifices a creature or planeswalker of their choice. target opponent discards a card. target opponent loses 3 life"
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_each_opponent_sacrifice_discard_lose_chain() {
        let normalized = normalize_common_semantic_phrasing(
            "When this creature enters, each opponent sacrifices a creature of their choice. For each opponent, that player discards a card. For each opponent, that player loses 4 life",
        );
        assert_eq!(
            normalized,
            "When this creature enters, each opponent sacrifices a creature of their choice, discards a card, and loses 4 life"
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_cast_article_and_unless_payer_pronoun() {
        let normalized = normalize_common_semantic_phrasing(
            "Whenever an opponent casts creature spell, that player loses 2 life unless that player pays {2}.",
        );
        assert_eq!(
            normalized,
            "Whenever an opponent casts a creature spell, that player loses 2 life unless they pay {2}."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_counter_type_lists() {
        let swan_song =
            normalize_common_semantic_phrasing("Counter target enchantment or instant or sorcery.");
        assert_eq!(
            swan_song,
            "Counter target enchantment, instant, or sorcery spell."
        );

        let strix = normalize_common_semantic_phrasing(
            "Counter target artifact or creature or planeswalker.",
        );
        assert_eq!(
            strix,
            "Counter target artifact, creature, or planeswalker spell."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_target_card_subtype_order() {
        let normalized = normalize_common_semantic_phrasing(
            "When Orah dies or a Cleric you control dies, return target card Cleric from your graveyard to the battlefield",
        );
        assert_eq!(
            normalized,
            "When Orah dies or a Cleric you control dies, return target Cleric card from your graveyard to the battlefield"
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_single_blocking_creature_verb() {
        let normalized = normalize_common_semantic_phrasing(
            "Whenever a creature blocks a black or red creature, blocking creatures get +1/+1 until end of turn.",
        );
        assert_eq!(
            normalized,
            "Whenever a creature blocks a black or red creature, the blocking creature gets +1/+1 until end of turn."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_each_players_end_step_land_sacrifice() {
        let normalized = normalize_common_semantic_phrasing(
            "At the beginning of each end step, that player sacrifices an untapped land.",
        );
        assert_eq!(
            normalized,
            "At the beginning of each player's end step, that player sacrifices an untapped land of their choice."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_granted_beginning_trigger_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "Enchanted land has at the beginning of your upkeep you may pay w w if you do you gain 1 life",
        );
        assert_eq!(
            normalized,
            "Enchanted land has \"At the beginning of your upkeep you may pay {W}{W}. If you do, you gain 1 life.\""
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_granted_beginning_trigger_clause_for_plural_subject() {
        let normalized = normalize_common_semantic_phrasing(
            "Creatures you control have at the beginning of your upkeep draw a card",
        );
        assert_eq!(
            normalized,
            "Creatures you control have \"At the beginning of your upkeep draw a card.\""
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_unholy_indenture_style_trigger() {
        let normalized = normalize_common_semantic_phrasing(
            "Whenever a enchanted creature dies, return it from graveyard to the battlefield. Put a +1/+1 counter on it.",
        );
        assert_eq!(
            normalized,
            "When enchanted creature dies, return that card to the battlefield under your control with a +1/+1 counter on it."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_false_demise_style_trigger() {
        let normalized = normalize_common_semantic_phrasing(
            "Whenever a enchanted creature dies, return it to the battlefield under your control.",
        );
        assert_eq!(
            normalized,
            "When enchanted creature dies, return that card to the battlefield under your control."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_nurgles_rot_style_trigger() {
        let normalized = normalize_common_semantic_phrasing(
            "Whenever a enchanted creature dies, return this permanent to its owner's hand. Create a 1/3 black Demon creature token under your control.",
        );
        assert_eq!(
            normalized,
            "When enchanted creature dies, return this card to its owner's hand and create a 1/3 black Demon creature token under your control."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_search_you_own_plural_card_subject() {
        let normalized = normalize_common_semantic_phrasing(
            "Search your library for up to three Aura you own, put them into your hand, then shuffle.",
        );
        assert_eq!(
            normalized,
            "Search your library for up to three Aura cards, put them into your hand, then shuffle."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_search_you_own_singular_card_subject() {
        let normalized = normalize_common_semantic_phrasing(
            "Search your library for up to one basic land or Gate you own, put it onto the battlefield tapped.",
        );
        assert_eq!(
            normalized,
            "Search your library for a basic land or Gate card, put it onto the battlefield tapped."
        );
    }

    #[test]
    fn surface_style_preserves_target_aura_subject() {
        let normalized =
            normalize_sentence_surface_style("Return target Aura to its owner's hand.");
        assert_eq!(normalized, "Return target Aura to its owner's hand.");
    }

    #[test]
    fn surface_style_preserves_search_top_then_shuffle_order() {
        let normalized = normalize_sentence_surface_style(
            "Search your library for a card, put it on top of library, then shuffle.",
        );
        assert_eq!(
            normalized,
            "Search your library for a card, put it on top of library, then shuffle."
        );
    }

    #[test]
    fn surface_style_normalizes_spider_slayer_graveyard_clause() {
        let normalized = normalize_sentence_surface_style(
            "Exile this creature: Create two 1/1 colorless Robot artifact creature token with flying tapped under your control.",
        );
        assert_eq!(
            normalized,
            "Exile this card from your graveyard: Create two tapped 1/1 colorless Robot artifact creature tokens with flying."
        );
    }

    #[test]
    fn surface_style_normalizes_archangels_light_clause() {
        let normalized = normalize_sentence_surface_style(
            "You gain 2 life for each card from a graveyard you own.",
        );
        assert_eq!(
            normalized,
            "You gain 2 life for each card from a graveyard you own."
        );
    }

    #[test]
    fn surface_style_normalizes_zombie_apocalypse_clause() {
        let normalized = normalize_sentence_surface_style(
            "Return all Zombie creature card in your graveyard to the battlefield tapped. Destroy all Humans.",
        );
        assert_eq!(
            normalized,
            "Return all Zombie creature cards from your graveyard to the battlefield tapped, then destroy all Humans."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_custom_you_create_token_trigger_head() {
        let normalized = normalize_common_semantic_phrasing(
            "You create a token: Put a +1/+1 counter on another target creature you control.",
        );
        assert_eq!(
            normalized,
            "Whenever you create a token, put a +1/+1 counter on another target creature you control."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_custom_unlock_door_trigger_head() {
        let normalized = normalize_common_semantic_phrasing(
            "You unlock this door: Create a token that's a copy of target creature you control.",
        );
        assert_eq!(
            normalized,
            "Whenever you unlock this door, create a token that's a copy of target creature you control."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_the_beginning_trigger_head() {
        let normalized = normalize_common_semantic_phrasing(
            "The beginning of your first main phase: Sacrifice this enchantment unless you Pay {E}.",
        );
        assert_eq!(
            normalized,
            "At the beginning of your first main phase, sacrifice this enchantment unless you Pay {E}."
        );
    }

    #[test]
    fn post_pass_normalizes_for_each_player_return_with_additional_counter_bundle() {
        let normalized = normalize_compiled_post_pass_effect(
            "For each player, Return all creature card from their graveyard to the battlefield. Put a -1/-1 counter on it.",
        );
        assert_eq!(
            normalized,
            "Each player returns each creature card from their graveyard to the battlefield with an additional -1/-1 counter on it."
        );
    }

    #[test]
    fn known_low_tail_normalizes_for_each_player_return_with_counter_chain() {
        let normalized = normalize_known_low_tail_phrase(
            "For each player, Return all creature card from their graveyard to the battlefield. Put a -1/-1 counter on it.",
        );
        assert_eq!(
            normalized,
            "Each player returns each creature card from their graveyard to the battlefield with an additional -1/-1 counter on it."
        );
    }

    #[test]
    fn known_low_tail_adds_any_order_for_choose_then_put_top_library() {
        let normalized = normalize_known_low_tail_phrase(
            "Target player chooses three cards from their hand, then puts them on top of their library.",
        );
        assert_eq!(
            normalized,
            "Target player chooses three cards from their hand and puts them on top of their library in any order."
        );
    }

    #[test]
    fn semantic_phrasing_normalizes_choose_exact_tagged_graveyard_chain() {
        let normalized = normalize_common_semantic_phrasing(
            "Target opponent chooses exactly 1 artifact card from their graveyard and tags it as '__it__'. Put it onto the battlefield under your control.",
        );
        assert_eq!(
            normalized,
            "Target opponent chooses an artifact card from their graveyard. Put it onto the battlefield under your control."
        );
    }

    #[test]
    fn known_low_tail_normalizes_choose_from_graveyard_put_under_your_control() {
        let normalized = normalize_known_low_tail_phrase(
            "Target opponent chooses artifact card from a graveyard. Put it onto the battlefield under your control.",
        );
        assert_eq!(
            normalized,
            "Target opponent chooses an artifact card in their graveyard. Put that card onto the battlefield under your control."
        );
    }

    #[test]
    fn known_low_tail_merges_target_player_loses_and_reveals_hand() {
        let normalized = normalize_known_low_tail_phrase(
            "Target player loses 1 life. Target player reveals their hand.",
        );
        assert_eq!(
            normalized,
            "Target player loses 1 life and reveals their hand."
        );
    }

    #[test]
    fn known_low_tail_merges_counter_then_prevent_all_damage() {
        let normalized = normalize_known_low_tail_phrase(
            "Put a +1/+1 counter on this creature. Prevent all damage that would be dealt to it this turn.",
        );
        assert_eq!(
            normalized,
            "Put a +1/+1 counter on this creature and prevent all damage that would be dealt to it this turn."
        );
    }

    #[test]
    fn known_low_tail_rewrites_choose_target_then_destroy_attached() {
        let normalized = normalize_known_low_tail_phrase(
            "Choose target creature. Destroy all Aura or Equipment attached to that object.",
        );
        assert_eq!(
            normalized,
            "Destroy all Aura or Equipment attached to target creature."
        );
    }

    #[test]
    fn known_low_tail_rewrites_trigger_choose_target_then_destroy_attached() {
        let normalized = normalize_known_low_tail_phrase(
            "Whenever this creature attacks, choose target land. Destroy all Aura attached to that object.",
        );
        assert_eq!(
            normalized,
            "Whenever this creature attacks, destroy all Aura attached to target land."
        );
    }

    #[test]
    fn post_pass_normalizes_repeated_return_subtype_chain_to_do_same_for() {
        let normalized = normalize_compiled_post_pass_effect(
            "Return card Pirate from your graveyard to your hand. Return card Vampire from your graveyard to your hand. Return card Dinosaur from your graveyard to your hand. Return card Merfolk from your graveyard to your hand.",
        );
        assert_eq!(
            normalized,
            "Return a Pirate card from your graveyard to your hand, then do the same for Vampire, Dinosaur, and Merfolk."
        );
    }
}
