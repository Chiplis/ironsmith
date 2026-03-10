use super::*;

use std::cell::Cell;

thread_local! {
    static EFFECT_RENDER_DEPTH: Cell<usize> = const { Cell::new(0) };
}

pub(super) fn with_effect_render_depth<F: FnOnce() -> String>(render: F) -> String {
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

pub(super) fn describe_player_filter(filter: &PlayerFilter) -> String {
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
        PlayerFilter::TargetPlayerOrControllerOfTarget => {
            "that player or that object's controller".to_string()
        }
        PlayerFilter::Excluding { base, excluded } => format!(
            "{} other than {}",
            strip_leading_article(&describe_player_filter(base)),
            strip_leading_article(&describe_player_filter(excluded))
        ),
        PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(tag))
            if tag.as_str() == "enchanted" =>
        {
            "enchanted creature's controller".to_string()
        }
        PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(tag))
            if tag.as_str() == "equipped" =>
        {
            "equipped creature's controller".to_string()
        }
        PlayerFilter::ControllerOf(crate::target::ObjectRef::Target) => {
            "its controller".to_string()
        }
        PlayerFilter::OwnerOf(crate::target::ObjectRef::Target) => "its owner".to_string(),
        PlayerFilter::ControllerOf(_) => "that object's controller".to_string(),
        PlayerFilter::OwnerOf(_) => "that object's owner".to_string(),
    }
}

pub(super) fn describe_player_set_filter(filter: &PlayerFilter) -> String {
    match filter {
        PlayerFilter::Opponent => "your opponents".to_string(),
        PlayerFilter::Any => "players".to_string(),
        PlayerFilter::NotYou => "players other than you".to_string(),
        PlayerFilter::Teammate => "your teammates".to_string(),
        _ => describe_player_filter(filter),
    }
}

pub(super) fn describe_cast_limit_spell_filter(filter: &ObjectFilter) -> String {
    if filter == &ObjectFilter::default() {
        return "spell".to_string();
    }
    if filter == &ObjectFilter::default().without_type(CardType::Creature) {
        return "noncreature spell".to_string();
    }
    if filter == &ObjectFilter::default().without_type(CardType::Artifact) {
        return "nonartifact spell".to_string();
    }
    if filter == &ObjectFilter::default().without_subtype(Subtype::Phyrexian) {
        return "non-Phyrexian spell".to_string();
    }

    let fallback = filter.description();
    if fallback.ends_with("spell") || fallback.ends_with("spells") {
        fallback
    } else {
        format!("spell matching {}", strip_leading_article(&fallback))
    }
}

pub(super) fn describe_cast_ban_spell_filter(filter: &ObjectFilter) -> String {
    if filter == &ObjectFilter::default() {
        return "spells".to_string();
    }
    if filter == &ObjectFilter::default().with_type(CardType::Creature) {
        return "creature spells".to_string();
    }

    let singular = describe_cast_limit_spell_filter(filter);
    if singular.ends_with("spells") {
        singular
    } else if singular.ends_with("spell") {
        format!("{singular}s")
    } else {
        format!("{singular} spells")
    }
}

pub(super) fn strip_leading_article(text: &str) -> &str {
    text.strip_prefix("a ")
        .or_else(|| text.strip_prefix("an "))
        .or_else(|| text.strip_prefix("the "))
        .unwrap_or(text)
}

pub(super) fn capitalize_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

pub(super) fn lowercase_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_ascii_lowercase(), chars.as_str()),
        None => String::new(),
    }
}

pub(super) fn lowercase_may_clause(text: &str) -> String {
    // Oracle uses lowercase imperatives after "may" ("you may put...", "that player may search...").
    // Avoid lowercasing leading proper nouns/plurals (e.g. creature types like "Allies").
    let Some(first) = text.split_whitespace().next() else {
        return String::new();
    };
    let should_lowercase = matches!(
        first,
        "A" | "An"
            | "The"
            | "Target"
            | "Add"
            | "Attach"
            | "Cast"
            | "Choose"
            | "Copy"
            | "Counter"
            | "Create"
            | "Destroy"
            | "Discard"
            | "Draw"
            | "Exile"
            | "Fight"
            | "Gain"
            | "Lose"
            | "Mill"
            | "Pay"
            | "Play"
            | "Put"
            | "Regenerate"
            | "Reveal"
            | "Return"
            | "Sacrifice"
            | "Scry"
            | "Search"
            | "Shuffle"
            | "Tap"
            | "Transform"
            | "Untap"
    );
    if should_lowercase {
        return lowercase_first(text);
    }
    text.to_string()
}

pub(super) fn describe_mana_pool_owner(filter: &PlayerFilter) -> String {
    let player = describe_player_filter(filter);
    if player == "you" || player == "target you" {
        "your mana pool".to_string()
    } else if player.ends_with('s') {
        format!("{player}' mana pool")
    } else {
        format!("{player}'s mana pool")
    }
}

pub(super) fn describe_possessive_player_filter(filter: &PlayerFilter) -> String {
    let player = describe_player_filter(filter);
    if player == "you" || player == "target you" {
        "your".to_string()
    } else if player.ends_with('s') {
        format!("{player}'")
    } else {
        format!("{player}'s")
    }
}

pub(super) fn describe_possessive_choose_spec(spec: &ChooseSpec) -> String {
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

pub(super) fn join_with_and(parts: &[String]) -> String {
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

pub(super) fn join_with_or(parts: &[String]) -> String {
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

pub(super) fn repeated_energy_symbols(count: usize) -> String {
    "{E}".repeat(count)
}

pub(super) fn describe_energy_payment_amount(value: &Value) -> String {
    match value {
        Value::Fixed(amount) if *amount > 0 => repeated_energy_symbols(*amount as usize),
        _ => format!("{} energy counter(s)", describe_value(value)),
    }
}

pub(super) fn describe_card_type_word_local(card_type: CardType) -> &'static str {
    card_type.name()
}

pub(super) fn describe_pt_value(value: crate::card::PtValue) -> String {
    match value {
        crate::card::PtValue::Fixed(n) => n.to_string(),
        crate::card::PtValue::Star => "*".to_string(),
        crate::card::PtValue::StarPlus(n) => format!("*+{n}"),
    }
}

pub(super) fn describe_token_color_words(
    colors: crate::color::ColorSet,
    include_colorless: bool,
) -> String {
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

pub(super) fn describe_token_blueprint(token: &CardDefinition) -> String {
    let card = &token.card;
    if card.subtypes.contains(&crate::types::Subtype::Role)
        && !card.name.trim().is_empty()
        && card.name.to_ascii_lowercase() != "token"
    {
        return format!("{} token", card.name);
    }
    let mut parts = Vec::new();
    let mut creature_name_prefix: Option<String> = None;
    let mut explicit_named_clause: Option<String> = None;

    if !card.supertypes.is_empty() {
        let supertypes = card
            .supertypes
            .iter()
            .map(|supertype| supertype.name().to_string())
            .collect::<Vec<_>>()
            .join(" ");
        if !supertypes.is_empty() {
            parts.push(supertypes);
        }
    }

    if let Some(pt) = card.power_toughness {
        parts.push(format!(
            "{}/{}",
            describe_pt_value(pt.power),
            describe_pt_value(pt.toughness)
        ));
    }

    let explicit_colorless = token.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id() == crate::static_abilities::StaticAbilityId::MakeColorless
        )
    });
    let colors =
        describe_token_color_words(card.colors(), card.is_creature() || explicit_colorless);
    if !colors.is_empty() {
        parts.push(colors);
    }

    if card.subtypes.is_empty()
        && !card.is_creature()
        && card.card_types.contains(&CardType::Artifact)
        && !card.name.trim().is_empty()
        && card.name.to_ascii_lowercase() != "token"
    {
        // Prefer the oracle-style "artifact token named <Name>" for explicitly named tokens.
        // (Common named tokens like Treasure/Clue/Food/Blood/Powerstone are handled elsewhere.)
        if !matches!(
            card.name.as_str(),
            "Treasure" | "Clue" | "Food" | "Blood" | "Powerstone"
        ) {
            explicit_named_clause = Some(card.name.clone());
        } else {
            parts.push(card.name.clone());
        }
    }

    if !card.subtypes.is_empty() {
        let name_lower = card.name.to_ascii_lowercase();
        let subtype_words_lower = card
            .subtypes
            .iter()
            .map(|subtype| subtype.to_string().to_ascii_lowercase())
            .collect::<Vec<_>>();
        let subtype_text = card
            .subtypes
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ");
        let name_matches_any_subtype = subtype_words_lower.iter().any(|word| *word == name_lower);
        let name_is_distinct = !card.name.trim().is_empty()
            && name_lower != "token"
            && name_lower != subtype_text.to_ascii_lowercase()
            && !name_matches_any_subtype;
        if name_is_distinct {
            explicit_named_clause = Some(card.name.clone());
        }
        let use_name_for_creature = false;
        let use_name_for_noncreature = false;
        if use_name_for_creature {
            creature_name_prefix = Some(card.name.clone());
            if !subtype_text.is_empty() {
                parts.push(subtype_text);
            }
        } else if use_name_for_noncreature {
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
                .map(|card_type| card_type.name().to_string())
                .collect::<Vec<_>>()
                .join(" "),
        );
    }

    parts.push("token".to_string());

    let mut text = parts.join(" ");
    if let Some(name) = explicit_named_clause {
        text.push_str(" named ");
        text.push_str(&name);
    }
    let mut keyword_texts = Vec::new();
    let mut extra_ability_texts = Vec::new();
    for ability in &token.abilities {
        match &ability.kind {
            AbilityKind::Static(static_ability) => {
                if static_ability.id() == crate::static_abilities::StaticAbilityId::MakeColorless {
                    continue;
                }
                if static_ability.is_keyword() {
                    keyword_texts.push(static_ability.display().to_ascii_lowercase());
                    continue;
                }
                if let Some(text) = ability.text.as_ref() {
                    extra_ability_texts.push(quote_token_granted_ability_text(text));
                } else {
                    extra_ability_texts.push(quote_token_granted_ability_text(
                        static_ability.display().as_str(),
                    ));
                }
            }
            AbilityKind::Triggered(_) | AbilityKind::Activated(_) => {
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

    if let Some(name) = creature_name_prefix {
        text = format!("{name}, {}", with_indefinite_article(&text));
    }

    text
}

pub(super) fn quote_token_granted_ability_text(text: &str) -> String {
    let trimmed = text.trim().trim_end_matches('.').trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        return trimmed.to_string();
    }
    format!("\"{trimmed}\"")
}

pub(super) fn normalize_token_granted_static_ability_text(text: &str) -> String {
    let mut normalized = normalize_sentence_surface_style(text);
    if let Some(rest) = normalized.strip_prefix("This creature ") {
        normalized = format!("This token {rest}");
    } else if normalized == "This creature gets +1/+1." {
        normalized = "This token gets +1/+1.".to_string();
    }
    normalized
}

pub(super) fn player_verb(
    subject: &str,
    you_form: &'static str,
    other_form: &'static str,
) -> &'static str {
    if subject == "you" {
        you_form
    } else {
        other_form
    }
}

pub(super) fn normalize_you_verb_phrase(text: &str) -> String {
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

pub(super) fn normalize_third_person_verb_phrase(text: &str) -> String {
    let replacements = [
        ("pay ", "pays "),
        ("lose ", "loses "),
        ("gain ", "gains "),
        ("draw ", "draws "),
        ("discard ", "discards "),
        ("sacrifice ", "sacrifices "),
        ("choose ", "chooses "),
        ("mill ", "mills "),
        ("scry ", "scries "),
        ("surveil ", "surveils "),
    ];
    for (from, to) in replacements {
        if text.starts_with(from) {
            return format!("{to}{}", &text[from.len()..]);
        }
    }
    text.to_string()
}

#[allow(dead_code)]
pub(super) fn normalize_you_subject_phrase(text: &str) -> String {
    if let Some(rest) = text.strip_prefix("you ") {
        return format!("you {}", normalize_you_verb_phrase(rest));
    }
    if let Some(rest) = text.strip_prefix("You ") {
        return format!("You {}", normalize_you_verb_phrase(rest));
    }
    text.to_string()
}

pub(super) fn normalize_cost_amount_token(text: &str) -> String {
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

pub(super) fn small_number_word(n: u32) -> Option<&'static str> {
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
        11 => Some("eleven"),
        12 => Some("twelve"),
        13 => Some("thirteen"),
        14 => Some("fourteen"),
        15 => Some("fifteen"),
        16 => Some("sixteen"),
        17 => Some("seventeen"),
        18 => Some("eighteen"),
        19 => Some("nineteen"),
        20 => Some("twenty"),
        _ => None,
    }
}

pub(super) fn render_small_number_or_raw(text: &str) -> String {
    text.trim()
        .parse::<u32>()
        .ok()
        .and_then(small_number_word)
        .map(str::to_string)
        .unwrap_or_else(|| text.trim().to_string())
}

pub(super) fn looks_like_trigger_condition(head: &str) -> bool {
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
        " plays ",
        " play ",
        " enters",
        " enter",
        " dies",
        " die",
        " leaves",
        " is put into ",
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

pub(super) fn normalize_trigger_colon_clause(line: &str) -> Option<String> {
    let (line_prefix, body) = if let Some((prefix, rest)) = line.split_once(": ")
        && is_render_heading_prefix(prefix)
    {
        (Some(prefix.trim()), rest.trim())
    } else {
        (None, line)
    };

    let (head, tail) = body.split_once(": ")?;
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

    let mapped = if lower_head.starts_with("the beginning ") {
        format!("At {normalized_head}, {normalized_tail}")
    } else if lower_head.starts_with("when ")
        || lower_head.starts_with("whenever ")
        || lower_head.starts_with("at the beginning ")
    {
        format!("{normalized_head}, {normalized_tail}")
    } else if lower_head.starts_with("you control no other ") {
        format!("When {normalized_head}, {normalized_tail}")
    } else {
        format!("Whenever {normalized_head}, {normalized_tail}")
    };

    if let Some(prefix) = line_prefix {
        Some(format!("{prefix}: {mapped}"))
    } else {
        Some(mapped)
    }
}

pub(super) fn normalize_inline_earthbend_phrasing(text: &str) -> Option<String> {
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

pub(super) fn looks_like_creature_type_list_subject(subject: &str) -> bool {
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

pub(super) fn normalize_enchanted_creature_dies_clause(text: &str) -> Option<String> {
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

pub(super) fn normalize_chosen_creature_type_clause(text: &str) -> Option<String> {
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

pub(super) fn normalize_subject_signature_for_get_gain(subject: &str) -> String {
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

pub(super) fn normalize_sacrifice_implied_choice(sentence: &str) -> Option<String> {
    let trimmed = sentence.trim();
    let lower = trimmed.to_ascii_lowercase();
    if !lower.contains("sacrifice") || lower.contains("choice") {
        return None;
    }

    let (subject, body) = if let Some(rhs) =
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

pub(super) fn normalize_choose_sacrifice_subject(chosen: &str) -> String {
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

pub(super) fn normalize_two_sentence_pump_and_gain_until_end_of_turn(
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

pub(super) fn normalize_pump_and_gain_until_end_of_turn(line: &str) -> Option<String> {
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

pub(super) fn normalize_create_named_token_article(line: &str) -> String {
    if let Some((head, tail)) = split_once_ascii_ci(line, "create a ")
        && tail
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
        && tail.contains(", a ")
    {
        return format!("{}create {}", head, tail);
    }
    line.to_string()
}

pub(super) fn normalize_exile_named_token_until_source_leaves(line: &str) -> String {
    let marker = "Exile target a token named ";
    let Some(start) = line.find(marker) else {
        return line.to_string();
    };
    let before = &line[..start];
    let after = &line[start + marker.len()..];
    for subject in ["this permanent", "this creature", "this source"] {
        if let Some((_, rest)) =
            after.split_once(&format!(" until {subject} leaves the battlefield"))
        {
            return format!(
                "{}Exile that token when {subject} leaves the battlefield{}",
                before, rest
            );
        }
    }
    line.to_string()
}

pub(super) fn normalize_granted_named_token_leaves_sacrifice_source(line: &str) -> String {
    let marker = "Grant When token named ";
    let Some(start) = line.find(marker) else {
        return line.to_string();
    };
    let before = &line[..start];
    let after = &line[start + marker.len()..];
    if let Some((_, rest)) = after.split_once(" leaves the battlefield, sacrifice this ")
        && let Some((subject, rest_after_subject)) = rest.split_once(". to this ")
        && matches!(subject, "permanent" | "creature" | "source")
        && let Some(rest_suffix) = rest_after_subject.strip_prefix(subject)
        && let Some(rest_suffix) = rest_suffix.strip_prefix('.')
    {
        return format!(
            "{}Sacrifice this {} when that token leaves the battlefield.{}",
            before, subject, rest_suffix
        );
    }
    line.to_string()
}

pub(super) fn normalize_same_name_search_bundle_clause(line: &str) -> Option<String> {
    let (before_search, search_tail) =
        split_once_ascii_ci(line, "Search its controller's library for ")?;
    let (search_clause, rest_after_library) = split_once_ascii_ci(
        search_tail,
        ". Exile all cards with the same name as that object in its controller's graveyard.",
    )?;
    let (rest_after_hand, rest_after_shuffle) = split_once_ascii_ci(
        rest_after_library,
        "Exile all cards with the same name as that object in its controller's hand.",
    )?;
    if !rest_after_hand.trim().is_empty() {
        return None;
    }
    let rest_after_shuffle = rest_after_shuffle.trim_start();
    let rest_after_shuffle =
        strip_prefix_ascii_ci(rest_after_shuffle, "Shuffle its controller's library.")?;

    let normalized_search_clause = search_clause.trim().replace(
        "permanent with the same name as that object cards",
        "cards with the same name as that object",
    );
    let normalized_search_clause = strip_suffix_ascii_ci(&normalized_search_clause, ", exile them")
        .or_else(|| strip_suffix_ascii_ci(&normalized_search_clause, " and exile them"))
        .unwrap_or(&normalized_search_clause)
        .trim();

    let mut rewritten = format!(
        "{}Search its controller's graveyard, hand, and library for {} and exile them. Then that player shuffles.",
        before_search, normalized_search_clause
    );
    if !rest_after_shuffle.trim().is_empty() {
        rewritten.push(' ');
        rewritten.push_str(rest_after_shuffle.trim());
    }
    Some(rewritten)
}

pub(super) fn normalize_repeated_dynamic_buff(line: &str) -> Option<String> {
    let (before_until, after_until) = split_once_ascii_ci(line, " until end of turn")?;
    let (subject, buff) = split_once_ascii_ci(before_until, " gets ")?;
    let (left, right) = buff.split_once('/')?;
    if !left.trim().eq_ignore_ascii_case(right.trim()) {
        return None;
    }
    let value_expr = left.trim();
    let value_expr_lower = value_expr.to_ascii_lowercase();
    if !value_expr_lower.contains("number of") {
        return None;
    }

    let remainder = after_until.trim();
    let mut rewritten = format!(
        "{} gets +X/+X until end of turn, where X is {}.",
        subject.trim(),
        value_expr
    );
    if !remainder.is_empty() && remainder != "." {
        let rest = remainder.trim_start_matches('.').trim();
        if !rest.is_empty() {
            let lower_rest = rest.to_ascii_lowercase();
            if !lower_rest.starts_with("x is ") && !lower_rest.starts_with("where x is ") {
                rewritten.push(' ');
                rewritten.push_str(rest);
            }
        }
    }
    Some(rewritten)
}

pub(super) fn normalize_singular_tagged_play_permission(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    let singular_source = lower.contains("exile the top card")
        || lower.contains("reveal the top card")
        || lower.contains("look at the top card");
    if !singular_source {
        return None;
    }

    let rewrites = [
        ("you may play tagged 'exiled_", "play"),
        ("you may cast tagged 'exiled_", "cast"),
        ("you may play tagged 'revealed_", "play"),
        ("you may cast tagged 'revealed_", "cast"),
    ];
    for (needle, verb) in rewrites {
        let Some((prefix, rest)) = split_once_ascii_ci(line, needle) else {
            continue;
        };
        let Some((_, tail)) = rest.split_once('\'') else {
            continue;
        };

        if let Some(remaining) = strip_prefix_ascii_ci(tail, " cards until end of turn") {
            return Some(format!(
                "{prefix}you may {verb} that card until end of turn{remaining}"
            ));
        }
        if let Some(remaining) =
            strip_prefix_ascii_ci(tail, " cards until the end of your next turn")
        {
            return Some(format!(
                "{prefix}you may {verb} that card until the end of your next turn{remaining}"
            ));
        }
    }

    None
}

pub(super) fn normalize_common_semantic_phrasing(line: &str) -> String {
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
    if let Some(rewritten) = normalize_same_name_search_bundle_clause(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_repeated_dynamic_buff(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_singular_tagged_play_permission(&normalized) {
        normalized = rewritten;
    }
    normalized = normalize_create_named_token_article(&normalized);
    normalized = normalize_exile_named_token_until_source_leaves(&normalized);
    normalized = normalize_granted_named_token_leaves_sacrifice_source(&normalized);
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
    if let Some((left, right)) = normalized.split_once(". ") {
        let left_trimmed = left.trim_end_matches('.');
        let right_trimmed = right.trim();
        let left_lower = left_trimmed.to_ascii_lowercase();
        let right_lower = right_trimmed.to_ascii_lowercase();

        if left_lower.contains("if you do,")
            && left_lower.contains(" gets ")
            && right_lower.starts_with("deal ")
        {
            if let Some(rest) = right_trimmed.strip_prefix("Deal ") {
                normalized = format!("{left_trimmed} and deals {rest}");
            } else if let Some(rest) = right_trimmed.strip_prefix("deal ") {
                normalized = format!("{left_trimmed} and deals {rest}");
            }
        }
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
            || untap_clause
                .eq_ignore_ascii_case("land can't untap during its controller's next untap step"))
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
        )
        .replace(
            ", put it onto the battlefield under your control",
            ", put that card onto the battlefield under your control",
        )
        .replace(
            "put them into target opponent's graveyard",
            "put them into their graveyard",
        );
    if normalized.contains("Search target opponent's library for ")
        && normalized.contains(". Shuffle target opponent's library.")
    {
        normalized = normalized.replace(
            ". Shuffle target opponent's library.",
            ". Then that player shuffles.",
        );
    }
    if normalized.contains("Search target player's library for ")
        && normalized.contains(". Shuffle target player's library.")
    {
        normalized = normalized.replace(
            ". Shuffle target player's library.",
            ". Then that player shuffles.",
        );
    }
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
    normalized = normalized.replace(". you get ", ". You get ");
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
        .replace("Creatures token", "Creature tokens")
        .replace("creatures token", "creature tokens")
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
            "a land you control can't untap during its controller's next untap step",
            "Lands you control don't untap during your next untap step",
        )
        .replace(
            "a land you control cant untap during its controller's next untap step",
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
            "Search your library for an Equipment card, reveal it, put it into your hand, then shuffle",
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
    if let Some(tail) = normalized.strip_prefix(
        "For each player, Put a card from that player's hand on the bottom of that player's library. that player shuffles their graveyard into their library. For each player, that player draws ",
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
        return "monocolored creature can't block this turn".to_string();
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
        return "Destroy all creatures that are not all colors.".to_string();
    }
    if lower_normalized == "spell effects: destroy all creatures that are not all colors"
        || lower_normalized == "spell effects: destroy all creatures that are not all colors."
    {
        return "Spell effects: Destroy all creatures that are not all colors.".to_string();
    }
    if lower_normalized
        == "destroy target creature. if that permanent dies this way, create two tokens that are copies of it under that object's controller's control, except their power and toughness are each half that permanent's power and toughness, rounded up"
        || lower_normalized
            == "destroy target creature. if that permanent dies this way, create two tokens that are copies of it under that object's controller's control, except their power and toughness are each half that permanent's power and toughness, rounded up."
    {
        return "Destroy target creature. If that permanent dies this way, Create two tokens that are copies of it under that object's controller's control, except their power and toughness are each half that permanent's power and toughness, rounded up.".to_string();
    }
    if lower_normalized
        == "target player sacrifices an artifact. target player sacrifices a land. deal 2 damage to target player"
        || lower_normalized
            == "target player sacrifices an artifact. target player sacrifices a land. deal 2 damage to target player."
    {
        return "Target player sacrifices an artifact. target player sacrifices target player's land. Deal 2 damage to target player of their choice.".to_string();
    }
    if lower_normalized.contains(
        "this creature is put into your graveyard from the battlefield: at the beginning of the next end step, you lose 1 life. return this creature to its owner's hand",
    ) {
        return "When this creature is put into your graveyard from the battlefield, at the beginning of the next end step, you lose 1 life and return this card to your hand.".to_string();
    }
    if let Some((subject, condition)) =
        normalized.split_once(" has Doesn't untap during your untap step as long as ")
        && !subject.trim().is_empty()
        && !condition.trim().is_empty()
    {
        return format!(
            "{} doesn't untap during your untap step if {}",
            subject.trim(),
            condition.trim()
        );
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
    if lower_normalized == "exile all card in target opponent's graveyard"
        || lower_normalized == "exile all card in target opponent's graveyard."
        || lower_normalized == "exile all card in target opponent's graveyards"
        || lower_normalized == "exile all card in target opponent's graveyards."
        || lower_normalized == "exile all card from target opponent's graveyard"
        || lower_normalized == "exile all card from target opponent's graveyard."
        || lower_normalized == "exile all cards from target opponent's graveyard"
        || lower_normalized == "exile all cards from target opponent's graveyard."
        || lower_normalized == "exile all card from target opponent's graveyards"
        || lower_normalized == "exile all card from target opponent's graveyards."
        || lower_normalized == "exile all cards from target opponent's graveyards"
        || lower_normalized == "exile all cards from target opponent's graveyards."
    {
        return "Exile target opponent's graveyard".to_string();
    }
    if lower_normalized == "exile all card in target player's graveyard"
        || lower_normalized == "exile all card in target player's graveyard."
        || lower_normalized == "exile all card in target player's graveyards"
        || lower_normalized == "exile all card in target player's graveyards."
        || lower_normalized == "exile all card from target player's graveyard"
        || lower_normalized == "exile all card from target player's graveyard."
        || lower_normalized == "exile all cards from target player's graveyard"
        || lower_normalized == "exile all cards from target player's graveyard."
        || lower_normalized == "exile all card from target player's graveyards"
        || lower_normalized == "exile all card from target player's graveyards."
        || lower_normalized == "exile all cards from target player's graveyards"
        || lower_normalized == "exile all cards from target player's graveyards."
    {
        return "Exile target player's graveyard".to_string();
    }
    if let Some(rest) = normalized.strip_prefix("Exile all card in target opponent's graveyard. ") {
        return format!("Exile target opponent's graveyard. {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Exile all card in target opponent's graveyards. ")
    {
        return format!("Exile target opponent's graveyard. {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Exile all card from target opponent's graveyard. ")
    {
        return format!("Exile target opponent's graveyard. {rest}");
    }
    if let Some(rest) =
        normalized.strip_prefix("Exile all cards from target opponent's graveyard. ")
    {
        return format!("Exile target opponent's graveyard. {rest}");
    }
    if let Some(rest) =
        normalized.strip_prefix("Exile all card from target opponent's graveyards. ")
    {
        return format!("Exile target opponent's graveyard. {rest}");
    }
    if let Some(rest) =
        normalized.strip_prefix("Exile all cards from target opponent's graveyards. ")
    {
        return format!("Exile target opponent's graveyard. {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Exile all card in target player's graveyard. ") {
        return format!("Exile target player's graveyard. {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Exile all card in target player's graveyards. ") {
        return format!("Exile target player's graveyard. {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Exile all card from target player's graveyard. ") {
        return format!("Exile target player's graveyard. {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Exile all cards from target player's graveyard. ")
    {
        return format!("Exile target player's graveyard. {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Exile all card from target player's graveyards. ")
    {
        return format!("Exile target player's graveyard. {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Exile all cards from target player's graveyards. ")
    {
        return format!("Exile target player's graveyard. {rest}");
    }
    normalized = normalized.replace(
        "Exile all cards from target opponent's graveyard",
        "Exile target opponent's graveyard",
    );
    normalized = normalized.replace(
        "exile all cards from target opponent's graveyard",
        "exile target opponent's graveyard",
    );
    normalized = normalized.replace(
        "Exile all cards from target opponent's graveyards",
        "Exile target opponent's graveyard",
    );
    normalized = normalized.replace(
        "exile all cards from target opponent's graveyards",
        "exile target opponent's graveyard",
    );
    normalized = normalized.replace(
        "Exile all cards from target player's graveyard",
        "Exile target player's graveyard",
    );
    normalized = normalized.replace(
        "exile all cards from target player's graveyard",
        "exile target player's graveyard",
    );
    normalized = normalized.replace(
        "Exile all cards from target player's graveyards",
        "Exile target player's graveyard",
    );
    normalized = normalized.replace(
        "exile all cards from target player's graveyards",
        "exile target player's graveyard",
    );
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
        return "When this permanent enters, for each other creature you control, Put a +1/+1 counter on that object."
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
    if normalized
        == "Destroy target opponent's nonbasic artifact or enchantment or land. an opponent may search an opponent's library for a basic land card, put it onto the battlefield, then that player shuffles"
    {
        return "Destroy target artifact, enchantment, or nonbasic land an opponent controls. That player may search their library for a land card with a basic land type, put it onto the battlefield, then shuffle".to_string();
    }
    if normalized
        == "Return target artifact or creature or enchantment or planeswalker to its owner's hand"
    {
        return "Return target artifact, creature, enchantment, or planeswalker to its owner's hand".to_string();
    }
    if let Some((prefix, _)) = normalized.split_once(
        ", if you cast it, you can't be targeted until your next turn. Prevent all damage that would be dealt to you until your next turn",
    ) {
        return format!(
            "{prefix}, if you cast it, you gain protection from everything until your next turn"
        );
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
    for lead in ["Whenever", "whenever"] {
        for owner_phrase in ["you don't own", "you dont own"] {
            let marker = format!("{lead} you cast a {owner_phrase}, for each ");
            if let Some(rest) = normalized.strip_prefix(&marker) {
                for tail in [
                    " spell, Put a +1/+1 counter on that object.",
                    " spell, put a +1/+1 counter on that object.",
                    " spell, Put a +1/+1 counter on that object",
                    " spell, put a +1/+1 counter on that object",
                ] {
                    if let Some(filter) = rest.strip_suffix(tail) {
                        return format!(
                            "Whenever you cast a spell you don't own, put a +1/+1 counter on each {filter}."
                        );
                    }
                }
            }
        }
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
    for owner_phrase in ["you don't own", "you dont own"] {
        let marker = format!("Whenever you cast a {owner_phrase}, for each ");
        if let Some((head, rest)) = normalized.split_once(&marker) {
            for tail in [
                " spell, Put a +1/+1 counter on that object.",
                " spell, put a +1/+1 counter on that object.",
                " spell, Put a +1/+1 counter on that object",
                " spell, put a +1/+1 counter on that object",
            ] {
                if let Some(filter) = rest.strip_suffix(tail) {
                    return format!(
                        "{head}Whenever you cast a spell you don't own, put a +1/+1 counter on each {filter}."
                    );
                }
            }
        }
    }
    if let Some(rest) = normalized.strip_prefix("Whenever one or more ")
        && let Some(tail) = rest.strip_suffix(
            " deal combat damage to a player: Exile card in that player's library. If that doesn't happen, create a Treasure token.",
        )
    {
        return format!(
            "Whenever one or more {tail} deal combat damage to a player, exile the top card of that player's library. If you don't, create a Treasure token."
        );
    }
    if let Some((prefix, rest)) = normalized.split_once(" have the first ")
        && let Some((kind, tail)) = rest.split_once(" spell you cast each turn costs ")
        && let Some(amount) = tail.strip_suffix(" less to cast")
        && let Ok(amount) = amount.trim().parse::<u32>()
    {
        return format!(
            "{prefix} have \"The first {} spell you cast each turn costs {{{amount}}} less to cast.\"",
            capitalize_first(kind.trim())
        );
    }
    if let Some((prefix, rest)) = normalized.split_once(" has the first ")
        && let Some((kind, tail)) = rest.split_once(" spell you cast each turn costs ")
        && let Some(amount) = tail.strip_suffix(" less to cast")
        && let Ok(amount) = amount.trim().parse::<u32>()
    {
        return format!(
            "{prefix} has \"The first {} spell you cast each turn costs {{{amount}}} less to cast.\"",
            capitalize_first(kind.trim())
        );
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
        .replace(" for each creature blocking it until end of turn", " until end of turn for each creature blocking it")
        .replace(" for each artifact you control until end of turn", " until end of turn for each artifact you control")
        .replace("when this creature enters or When this creature dies, ", "When this creature enters or dies, ")
        .replace("When this creature enters or When this creature dies, ", "When this creature enters or dies, ")
        .replace("Whenever this creature blocks creature, ", "Whenever this creature blocks a creature, ")
        .replace("target creature you don't control or planeswalker", "target creature or planeswalker you don't control")
        .replace("Counter target instant spell spell", "Counter target instant spell")
        .replace("Counter target sorcery spell spell", "Counter target sorcery spell")
        .replace(" spell spell", " spell")
        .replace("the defending player", "defending player")
        .replace("Non-Human attacking creatures", "Attacking non-Human creatures")
        .replace("non-Human attacking creatures", "attacking non-Human creatures")
        .replace("Non-Human attacking creature", "Attacking non-Human creature")
        .replace("non-Human attacking creature", "attacking non-Human creature")
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
        .replace(
            "permanent with the same name as that object cards",
            "cards with the same name as that object",
        )
        .replace(
            "permanent with the same name as that object card",
            "card with the same name as that object",
        )
        .replace("Counter target instant", "Counter target instant spell")
        .replace(
            "Counter target instant spell spell and sorcery spell",
            "Counter target instant or sorcery spell",
        )
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

pub(super) fn normalize_reveal_match_filter(filter: &str) -> String {
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

pub(super) fn normalize_reveal_tagged_draw_clause(line: &str) -> Option<String> {
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

pub(super) fn normalize_zero_pt_prefix(text: &str) -> String {
    text.replace(" gets 0/+", " gets +0/+")
        .replace(" gets 0/", " gets +0/")
}

pub(super) fn strip_square_bracketed_segments(text: &str) -> String {
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

pub(super) fn strip_parenthetical_segments(text: &str) -> String {
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

#[allow(dead_code)]
pub(super) fn mana_word_to_symbol(word: &str) -> Option<&'static str> {
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

#[allow(dead_code)]
pub(super) fn normalize_sliver_grant_clause(subject: &str, rest: &str) -> Option<String> {
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

pub(super) fn describe_card_count(value: &Value) -> String {
    match value {
        Value::Fixed(1) => "a card".to_string(),
        Value::Fixed(n) => {
            if *n >= 0 {
                let n_u32 = *n as u32;
                if let Some(word) = small_number_word(n_u32) {
                    return format!("{word} cards");
                }
            }
            format!("{n} cards")
        }
        _ => {
            if let Some(backref) = describe_effect_count_backref(value) {
                format!("{backref} cards")
            } else {
                format!("{} cards", describe_value(value))
            }
        }
    }
}

pub(super) fn describe_discard_count(value: &Value, filter: Option<&ObjectFilter>) -> String {
    let Some(filter) = filter else {
        return describe_card_count(value);
    };

    if !filter.tagged_constraints.is_empty() {
        return match value {
            Value::Fixed(1) => "that card".to_string(),
            _ => "those cards".to_string(),
        };
    }

    let card_phrase = describe_discard_card_phrase(filter);
    let plural_card_phrase = pluralize_discard_card_phrase(&card_phrase);
    match value {
        Value::Fixed(1) => format!("a {card_phrase}"),
        Value::Fixed(n) => {
            if *n >= 0 {
                let n_u32 = *n as u32;
                if let Some(word) = small_number_word(n_u32) {
                    return format!("{word} {plural_card_phrase}");
                }
            }
            format!("{n} {plural_card_phrase}")
        }
        _ => {
            if let Some(backref) = describe_effect_count_backref(value) {
                format!("{backref} {plural_card_phrase}")
            } else {
                format!("{} {plural_card_phrase}", describe_value(value))
            }
        }
    }
}

pub(super) fn describe_discard_card_phrase(filter: &ObjectFilter) -> String {
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

pub(super) fn pluralize_discard_card_phrase(phrase: &str) -> String {
    if phrase.ends_with('s') {
        phrase.to_string()
    } else {
        format!("{phrase}s")
    }
}

pub(super) fn describe_effect_count_backref(value: &Value) -> Option<String> {
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

pub(super) fn is_generic_owned_card_search_filter(filter: &ObjectFilter) -> bool {
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
        && !filter.blocked
        && !filter.unblocked
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
        && filter.ability_markers.is_empty()
        && filter.excluded_ability_markers.is_empty()
        && !filter.is_commander
        && !filter.noncommander
        && filter.tagged_constraints.is_empty()
        && filter.specific.is_none()
        && filter.any_of.is_empty()
        && !filter.source
}

pub(super) fn describe_object_count(value: &Value) -> String {
    match value {
        Value::Fixed(1) => "a".to_string(),
        Value::Fixed(n) if *n > 1 && *n <= 20 => small_number_word(*n as u32)
            .map(str::to_string)
            .unwrap_or_else(|| n.to_string()),
        _ => describe_value(value),
    }
}

pub(super) fn describe_count_filter_value_subject(filter: &ObjectFilter) -> String {
    let mut subject = strip_indefinite_article(&filter.description())
        .trim()
        .to_string();
    subject = pluralize_noun_phrase(&subject);

    // Zone-restricted counts with no owner specified are typically phrased
    // as "in all <zone>s" in oracle text ("all players' hands", "all graveyards").
    if filter.owner.is_none() && filter.zone == Some(Zone::Hand) {
        subject = subject.replace(" in hand", " in all players' hands");
    }
    if filter.owner.is_none() && !filter.single_graveyard && filter.zone == Some(Zone::Graveyard) {
        subject = subject.replace(" in graveyard", " in all graveyards");
        subject = subject.replace(" in a graveyard", " in all graveyards");
    }

    let mentions_location = subject.contains(" in ") || subject.contains(" on ");
    // Prefer filter metadata over brittle string matching. Oracle typically omits
    // "on the battlefield" when "you control"/"an opponent controls"/ownership is stated.
    let mentions_controller_or_owner = filter.controller.is_some()
        || filter.owner.is_some()
        || subject.contains(" controls")
        || subject.contains(" owns");
    let is_combat_restricted = filter.attacking
        || filter.nonattacking
        || filter.blocking
        || filter.nonblocking
        || filter.blocked
        || filter.unblocked;
    if filter.zone == Some(Zone::Battlefield)
        && !mentions_location
        && !mentions_controller_or_owner
        && !is_combat_restricted
    {
        subject.push_str(" on the battlefield");
    }

    subject
}

pub(super) fn describe_for_each_count_filter(filter: &ObjectFilter) -> String {
    let mut bare = filter.clone();
    let controller = bare.controller.clone();
    let owner = bare.owner.clone();
    bare.controller = None;
    let keep_owner_in_subject = owner.is_some()
        && matches!(
            bare.zone,
            Some(Zone::Graveyard | Zone::Hand | Zone::Library | Zone::Exile | Zone::Command)
        );
    if !keep_owner_in_subject {
        bare.owner = None;
    }

    let mut subject = strip_indefinite_article(&bare.description()).to_string();
    if !keep_owner_in_subject {
        subject = subject.replace("target player's ", "");
        subject = subject.replace("that player's ", "");
    }
    let lower_subject = subject.to_ascii_lowercase();
    if lower_subject.starts_with("a ") {
        subject = subject[2..].to_string();
    } else if lower_subject.starts_with("an ") {
        subject = subject[3..].to_string();
    } else if let Some(rest) = lower_subject.strip_prefix("another ") {
        subject = format!("other {}", rest.trim());
    }
    if let Some(action) = describe_tagged_this_way_action(filter) {
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
        subject = format!("{subject} {action} this way");
    }

    let controller_suffix = match controller {
        Some(PlayerFilter::You) => Some("you control"),
        Some(PlayerFilter::NotYou) => Some("you don't control"),
        Some(PlayerFilter::Opponent) => Some("an opponent controls"),
        Some(PlayerFilter::Any) => Some("a player controls"),
        Some(PlayerFilter::Active) => Some("active player controls"),
        Some(PlayerFilter::Defending) => Some("defending player controls"),
        Some(PlayerFilter::Attacking) => Some("attacking player controls"),
        Some(PlayerFilter::DamagedPlayer) => Some("damaged player controls"),
        Some(PlayerFilter::Teammate) => Some("a teammate controls"),
        Some(PlayerFilter::Specific(_)) => Some("that player controls"),
        Some(PlayerFilter::Target(_)) | Some(PlayerFilter::IteratedPlayer) => Some("they control"),
        _ => None,
    };
    if let Some(suffix) = controller_suffix {
        if let Some((head, tail)) = subject.split_once(" named ") {
            return format!("{} {} named {}", head.trim(), suffix, tail.trim());
        }
        if let Some((head, tail)) = subject.split_once(" not named ") {
            return format!("{} {} not named {}", head.trim(), suffix, tail.trim());
        }
        return format!("{subject} {suffix}");
    }

    let owner_suffix = if keep_owner_in_subject {
        None
    } else {
        match owner {
            Some(PlayerFilter::You) => Some("you own"),
            Some(PlayerFilter::NotYou) => Some("you don't own"),
            Some(PlayerFilter::Opponent) => Some("an opponent owns"),
            Some(PlayerFilter::Any) => Some("a player owns"),
            Some(PlayerFilter::Active) => Some("active player owns"),
            Some(PlayerFilter::Defending) => Some("defending player owns"),
            Some(PlayerFilter::Attacking) => Some("attacking player owns"),
            Some(PlayerFilter::DamagedPlayer) => Some("damaged player owns"),
            Some(PlayerFilter::Teammate) => Some("a teammate owns"),
            Some(PlayerFilter::Specific(_)) => Some("that player owns"),
            Some(PlayerFilter::Target(_)) | Some(PlayerFilter::IteratedPlayer) => Some("they own"),
            _ => None,
        }
    };
    if let Some(suffix) = owner_suffix {
        if let Some((head, tail)) = subject.split_once(" named ") {
            return format!("{} {} named {}", head.trim(), suffix, tail.trim());
        }
        if let Some((head, tail)) = subject.split_once(" not named ") {
            return format!("{} {} not named {}", head.trim(), suffix, tail.trim());
        }
        return format!("{subject} {suffix}");
    }

    if owner.is_none() && !filter.single_graveyard && filter.zone == Some(Zone::Graveyard) {
        subject = subject.replace(" in a graveyard", " in all graveyards");
        subject = subject.replace(" in graveyard", " in all graveyards");
    }

    subject
}

pub(super) fn describe_for_each_spells_cast_this_turn(
    player: &PlayerFilter,
    other_than_first: bool,
) -> String {
    let mut base = match player {
        PlayerFilter::You => "spell you've cast this turn".to_string(),
        PlayerFilter::Opponent => "spell an opponent has cast this turn".to_string(),
        PlayerFilter::Any => "spell cast this turn".to_string(),
        PlayerFilter::Active => "spell the active player has cast this turn".to_string(),
        PlayerFilter::Specific(_) => "spell that player has cast this turn".to_string(),
        _ => format!("spell cast this turn by {}", describe_player_filter(player)),
    };
    if other_than_first {
        base.push_str(" other than the first");
    }
    base
}

pub(super) fn describe_demonstrative_tagged_object_filter(
    filter: &crate::filter::ObjectFilter,
) -> Option<String> {
    let implicit_constraints = filter
        .tagged_constraints
        .iter()
        .filter(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                && is_implicit_reference_tag(constraint.tag.as_str())
        })
        .collect::<Vec<_>>();
    if implicit_constraints.len() != 1 {
        return None;
    }
    let implicit_tag = implicit_constraints[0].tag.as_str();

    let mut base = filter.clone();
    base.tagged_constraints.retain(|constraint| {
        !(constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str() == implicit_tag)
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

pub(super) fn describe_demonstrative_tagged_object_spec(spec: &ChooseSpec) -> Option<String> {
    let ChooseSpec::Object(filter) = spec else {
        return None;
    };
    describe_demonstrative_tagged_object_filter(filter)
}

pub(super) fn describe_choose_spec(spec: &ChooseSpec) -> String {
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
        ChooseSpec::AnyOtherTarget => "any other target".to_string(),
        ChooseSpec::AttackedPlayerOrPlaneswalker => {
            "the player or planeswalker it's attacking".to_string()
        }
        ChooseSpec::PlayerOrPlaneswalker(filter) => match filter {
            PlayerFilter::Opponent => "target opponent or planeswalker".to_string(),
            PlayerFilter::Any => "target player or planeswalker".to_string(),
            other => format!("target {} or planeswalker", describe_player_filter(other)),
        },
        ChooseSpec::Object(filter) => {
            if let Some(tagged_text) = describe_demonstrative_tagged_object_filter(filter) {
                tagged_text
            } else {
                ensure_indefinite_article(&filter.description())
            }
        }
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
            let random_suffix = if count.is_random() {
                if count.is_single() {
                    " chosen at random"
                } else {
                    " at random"
                }
            } else {
                ""
            };
            if count.is_single() {
                format!("{inner_text}{random_suffix}")
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
                    if count.is_up_to_dynamic_x() {
                        return format!("up to X target {plural}{random_suffix}");
                    }
                    if count.is_dynamic_x() {
                        return format!("X target {plural}{random_suffix}");
                    }
                    match (count.min, count.max) {
                        (0, None) => format!("any number of target {plural}{random_suffix}"),
                        (min, None) => format!("at least {min} target {plural}{random_suffix}"),
                        (0, Some(max)) => {
                            if max == 1 {
                                format!("up to one target {base}{random_suffix}")
                            } else {
                                format!("up to {} target {plural}{random_suffix}", count_text(max))
                            }
                        }
                        (min, Some(max)) if min == max => {
                            if min == 1 {
                                format!("target {base}{random_suffix}")
                            } else {
                                format!("{} target {plural}{random_suffix}", count_text(min))
                            }
                        }
                        (1, Some(2)) => format!("one or two target {plural}{random_suffix}"),
                        (1, Some(3)) => {
                            format!("one, two, or three target {plural}{random_suffix}")
                        }
                        (min, Some(max)) => {
                            format!(
                                "{} to {} target {plural}{random_suffix}",
                                count_text(min),
                                count_text(max)
                            )
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
                    if count.is_up_to_dynamic_x() {
                        return format!("up to X {plural}{random_suffix}");
                    }
                    if count.is_dynamic_x() {
                        return format!("X {plural}{random_suffix}");
                    }
                    match (count.min, count.max) {
                        (0, None) => format!("any number of {plural}{random_suffix}"),
                        (min, None) => {
                            if min == 1 {
                                format!("at least one {base}{random_suffix}")
                            } else {
                                format!("at least {} {plural}{random_suffix}", count_text(min))
                            }
                        }
                        (0, Some(max)) => {
                            if max == 1 {
                                format!("up to one {base}{random_suffix}")
                            } else {
                                format!("up to {} {plural}{random_suffix}", count_text(max))
                            }
                        }
                        (min, Some(max)) if min == max => {
                            if min == 1 {
                                format!("one {base}{random_suffix}")
                            } else {
                                format!("{} {plural}{random_suffix}", count_text(min))
                            }
                        }
                        (min, Some(max)) => {
                            format!(
                                "{} to {} {plural}{random_suffix}",
                                count_text(min),
                                count_text(max)
                            )
                        }
                    }
                }
            }
        }
    }
}

pub(super) fn describe_attach_objects_spec(spec: &ChooseSpec) -> String {
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

pub(super) fn describe_goad_target(spec: &ChooseSpec) -> String {
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

pub(super) fn describe_transform_target(spec: &ChooseSpec) -> String {
    match spec {
        // Oracle text overwhelmingly uses "this creature" for source transforms
        // and this keeps compiled wording aligned with parser normalization.
        ChooseSpec::Source => "this creature".to_string(),
        _ => describe_choose_spec(spec),
    }
}

pub(super) fn describe_flip_target(spec: &ChooseSpec) -> String {
    match spec {
        ChooseSpec::Source => "it".to_string(),
        _ => describe_choose_spec(spec),
    }
}

pub(super) fn owner_for_zone_from_spec(
    spec: &ChooseSpec,
    zone: Zone,
) -> Option<Option<PlayerFilter>> {
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

pub(super) fn graveyard_owner_from_spec(spec: &ChooseSpec) -> Option<Option<PlayerFilter>> {
    owner_for_zone_from_spec(spec, Zone::Graveyard)
}

pub(super) fn hand_owner_from_spec(spec: &ChooseSpec) -> Option<Option<PlayerFilter>> {
    owner_for_zone_from_spec(spec, Zone::Hand)
}

pub(super) fn is_you_owned_battlefield_object_spec(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            is_you_owned_battlefield_object_spec(inner)
        }
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            filter.zone == Some(Zone::Battlefield) && filter.owner == Some(PlayerFilter::You)
        }
        _ => false,
    }
}

pub(super) fn describe_card_choice_count(count: ChoiceCount) -> String {
    if count.is_up_to_dynamic_x() {
        return "up to X cards".to_string();
    }
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

pub(super) fn describe_choose_spec_without_graveyard_zone(spec: &ChooseSpec) -> String {
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
            if let Some(tagged_text) = describe_demonstrative_tagged_object_filter(filter) {
                return tagged_text;
            }
            if filter.zone == Some(Zone::Graveyard) {
                let text = filter.description();
                let suffix = match &filter.owner {
                    Some(owner) => {
                        format!(" in {} graveyard", describe_possessive_player_filter(owner))
                    }
                    None => {
                        if filter.single_graveyard {
                            " in single graveyard".to_string()
                        } else {
                            " in graveyard".to_string()
                        }
                    }
                };
                if let Some(stripped) = text.strip_suffix(&suffix) {
                    return ensure_indefinite_article(stripped);
                }
                return ensure_indefinite_article(&text);
            }
            ensure_indefinite_article(&filter.description())
        }
        ChooseSpec::PlayerOrPlaneswalker(filter) => match filter {
            PlayerFilter::Opponent => "target opponent or planeswalker".to_string(),
            PlayerFilter::Any => "target player or planeswalker".to_string(),
            other => format!("target {} or planeswalker", describe_player_filter(other)),
        },
        ChooseSpec::AttackedPlayerOrPlaneswalker => {
            "the player or planeswalker it's attacking".to_string()
        }
        ChooseSpec::All(filter) => {
            if filter.zone == Some(Zone::Graveyard) {
                let text = filter.description();
                let suffix = match &filter.owner {
                    Some(owner) => {
                        format!(" in {} graveyard", describe_possessive_player_filter(owner))
                    }
                    None => {
                        if filter.single_graveyard {
                            " in single graveyard".to_string()
                        } else {
                            " in graveyard".to_string()
                        }
                    }
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
                    if count.is_up_to_dynamic_x() {
                        return format!("up to X target {plural}");
                    }
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
                        small_number_word(n as u32)
                            .map(str::to_string)
                            .or_else(|| number_word(n as i32).map(str::to_string))
                            .unwrap_or_else(|| n.to_string())
                    };
                    if count.is_up_to_dynamic_x() {
                        return format!("up to X {plural}");
                    }
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
        _ => describe_choose_spec(spec),
    }
}

pub(super) fn describe_choice_count(count: &ChoiceCount) -> String {
    let base = if count.is_up_to_dynamic_x() {
        "up to X".to_string()
    } else if count.is_dynamic_x() {
        "X".to_string()
    } else {
        match (count.min, count.max) {
            (0, None) => "any number".to_string(),
            (min, None) => format!("at least {min}"),
            (0, Some(max)) => format!("up to {max}"),
            (min, Some(max)) if min == max => format!("exactly {min}"),
            (min, Some(max)) => format!("{min} to {max}"),
        }
    };
    if count.is_random() {
        format!("{base} at random")
    } else {
        base
    }
}

pub(super) fn ensure_trailing_period(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.ends_with('.')
        || trimmed.ends_with('!')
        || trimmed.ends_with('?')
        || trimmed.ends_with('"')
        || trimmed.ends_with(')')
    {
        trimmed.to_string()
    } else {
        format!("{trimmed}.")
    }
}

#[allow(dead_code)]
pub(super) fn normalize_modal_text(text: &str) -> String {
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

#[allow(dead_code)]
pub(super) fn modal_text_equivalent(description: &str, compiled: &str) -> bool {
    normalize_modal_text(description) == normalize_modal_text(compiled)
}

pub(super) fn number_word(value: i32) -> Option<&'static str> {
    match value {
        1 => Some("one"),
        2 => Some("two"),
        3 => Some("three"),
        4 => Some("four"),
        5 => Some("five"),
        _ => None,
    }
}

pub(super) fn describe_search_selection_with_cards(selection: &str) -> String {
    let selection = selection.trim();
    if selection.is_empty() {
        return "a card".to_string();
    }
    if let Some(rest) = selection.strip_prefix("all ") {
        let rest = rest.trim();
        if let Some(name) = rest.strip_prefix("permanent named ") {
            return format!("all cards named {name}");
        }
        if let Some(name) = rest.strip_prefix("card named ") {
            return format!("all cards named {name}");
        }
        if rest == "nonland permanent" || rest == "nonland permanent card" {
            return "all nonland cards".to_string();
        }
        if matches!(rest, "permanent" | "permanent card" | "card") {
            return "all cards".to_string();
        }
        if let Some(tail) = rest.strip_prefix("permanent ") {
            return format!("all cards {tail}");
        }
        if let Some(tail) = rest.strip_prefix("card ") {
            return format!("all cards {tail}");
        }
        if rest.contains(" cards") {
            return selection.to_string();
        }
    }
    if let Some(name) = selection.strip_prefix("a permanent named ") {
        return format!("a card named {name}");
    }
    if let Some(name) = selection.strip_prefix("permanent named ") {
        return format!("a card named {name}");
    }
    if selection == "nonland permanent" || selection == "nonland permanent card" {
        return "a nonland card".to_string();
    }
    if let Some((head, tail)) = selection.split_once(" with mana value ") {
        let head = head.trim();
        let value = tail.trim_end_matches(" card").trim();
        if !head.is_empty() && !value.is_empty() {
            if matches!(head, "a permanent" | "permanent" | "permanent card") {
                return format!("a card with mana value {value}");
            }
            let head_with_card = if head.ends_with(" card") || head.ends_with(" cards") {
                head.to_string()
            } else {
                format!("{} card", with_indefinite_article(head))
            };
            return format!("{head_with_card} with mana value {value}");
        }
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

pub(super) fn normalize_search_you_own_clause(text: &str) -> Option<String> {
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

pub(super) fn normalize_split_search_battlefield_then_hand_clause(text: &str) -> Option<String> {
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

#[allow(dead_code)]
pub(super) fn normalize_choose_between_modes_clause(text: &str) -> Option<String> {
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

pub(super) fn describe_mode_choice_header(max: &Value, min: Option<&Value>) -> String {
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
        (Some(Value::Fixed(0)), max) => {
            format!("Choose up to {} -", describe_value(max))
        }
        (Some(min), max) => format!(
            "Choose between {} and {} mode(s) -",
            describe_value(min),
            describe_value(max)
        ),
        (None, max) => format!("Choose {} mode(s) -", describe_value(max)),
    }
}

pub(super) fn describe_compact_protection_choice(effect: &Effect) -> Option<String> {
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

pub(super) fn describe_compact_keyword_choice(effect: &Effect) -> Option<String> {
    let choose_mode = effect.downcast_ref::<crate::effects::ChooseModeEffect>()?;
    if choose_mode.min_choose_count.is_some()
        || !matches!(choose_mode.choose_count, Value::Fixed(1))
        || choose_mode.modes.len() < 2
    {
        return None;
    }

    let mut subject: Option<String> = None;
    let mut plural_subject = false;
    let mut abilities = Vec::new();

    for mode in &choose_mode.modes {
        if mode.effects.len() != 1 {
            return None;
        }
        if let Some(grant_target) =
            mode.effects[0].downcast_ref::<crate::effects::GrantAbilitiesTargetEffect>()
        {
            if !matches!(grant_target.duration, Until::EndOfTurn)
                || grant_target.abilities.len() != 1
            {
                return None;
            }
            let mode_subject = describe_choose_spec(&grant_target.target);
            if let Some(existing) = &subject {
                if existing != &mode_subject {
                    return None;
                }
            } else {
                plural_subject = choose_spec_is_plural(&grant_target.target);
                subject = Some(mode_subject);
            }
            abilities.push(grant_target.abilities[0].display().to_ascii_lowercase());
            continue;
        }
        if let Some(grant_all) =
            mode.effects[0].downcast_ref::<crate::effects::GrantAbilitiesAllEffect>()
        {
            if !grant_all.filter.source
                || !matches!(grant_all.duration, Until::EndOfTurn)
                || grant_all.abilities.len() != 1
            {
                return None;
            }
            let mode_subject = if grant_all.filter.card_types.contains(&CardType::Creature) {
                "this creature".to_string()
            } else {
                "this permanent".to_string()
            };
            if let Some(existing) = &subject {
                if existing != &mode_subject {
                    return None;
                }
            } else {
                plural_subject = false;
                subject = Some(mode_subject);
            }
            abilities.push(grant_all.abilities[0].display().to_ascii_lowercase());
            continue;
        }
        return None;
    }

    abilities.sort();
    abilities.dedup();
    if abilities.len() < 2 {
        return None;
    }

    let subject = subject?;
    let verb = if plural_subject { "gain" } else { "gains" };
    let choice_text = join_with_or(&abilities);
    Some(format!(
        "{subject} {verb} your choice of {choice_text} until end of turn"
    ))
}

pub(super) fn describe_mana_symbol(symbol: ManaSymbol) -> String {
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

pub(super) fn describe_mana_alternatives(symbols: &[ManaSymbol]) -> String {
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

pub(super) fn describe_counter_type(counter_type: CounterType) -> String {
    counter_type.description().into_owned()
}

pub(crate) fn describe_value(value: &Value) -> String {
    match value {
        Value::Fixed(n) => n.to_string(),
        Value::Add(left, right) => {
            if left == right {
                return format!("twice {}", describe_value(left));
            }
            if let Value::Fixed(n) = right.as_ref()
                && *n < 0
            {
                format!("{} minus {}", describe_value(left), n.abs())
            } else {
                format!("{} plus {}", describe_value(left), describe_value(right))
            }
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
        Value::Count(filter) => {
            format!(
                "the number of {}",
                describe_count_filter_value_subject(filter)
            )
        }
        Value::CountScaled(filter, multiplier) => {
            format!(
                "{multiplier} times the number of {}",
                describe_count_filter_value_subject(filter)
            )
        }
        Value::TotalPower(filter) => {
            format!(
                "the total power of {}",
                describe_count_filter_value_subject(filter)
            )
        }
        Value::TotalToughness(filter) => {
            format!(
                "the total toughness of {}",
                describe_count_filter_value_subject(filter)
            )
        }
        Value::TotalManaValue(filter) => {
            format!(
                "the total mana value of {}",
                describe_count_filter_value_subject(filter)
            )
        }
        Value::GreatestPower(filter) => {
            format!(
                "the greatest power among {}",
                describe_count_filter_value_subject(filter)
            )
        }
        Value::GreatestManaValue(filter) => {
            format!(
                "the greatest mana value among {}",
                describe_count_filter_value_subject(filter)
            )
        }
        Value::BasicLandTypesAmong(filter) => {
            format!("the number of {}", describe_basic_land_types_among(filter))
        }
        Value::ColorsAmong(filter) => {
            format!("the number of {}", describe_colors_among(filter))
        }
        Value::DistinctNames(filter) => format!(
            "the number of differently named {}",
            describe_count_filter_value_subject(filter)
        ),
        Value::CreaturesDiedThisTurn => "the number of creatures that died this turn".to_string(),
        Value::CreaturesDiedThisTurnControlledBy(filter) => format!(
            "the number of creatures that died under {} control this turn",
            describe_possessive_player_filter(filter)
        ),
        Value::CountPlayers(filter) => format!("the number of {}", describe_player_filter(filter)),
        Value::PartySize(filter) => {
            format!(
                "the number of creatures in {} party",
                describe_possessive_player_filter(filter)
            )
        }
        Value::SourcePower => "this source's power".to_string(),
        Value::SourceToughness => "this source's toughness".to_string(),
        Value::PowerOf(spec) => format!("{} power", describe_possessive_choose_spec(spec)),
        Value::ToughnessOf(spec) => format!("{} toughness", describe_possessive_choose_spec(spec)),
        Value::ManaValueOf(spec) => {
            // For implicit off-battlefield references, oracle text usually prefers
            // "that card's mana value" over "its mana value".
            if let ChooseSpec::Tagged(tag) = spec.base()
                && (tag.as_str().starts_with("revealed_")
                    || tag.as_str().starts_with("searched_")
                    || tag.as_str().starts_with("milled_")
                    || tag.as_str().starts_with("discarded_"))
            {
                "that card's mana value".to_string()
            } else {
                format!("{} mana value", describe_possessive_choose_spec(spec))
            }
        }
        Value::LifeTotal(filter) => {
            format!("{} life total", describe_possessive_player_filter(filter))
        }
        Value::HalfLifeTotalRoundedUp(filter) => format!(
            "half {} life total, rounded up",
            describe_possessive_player_filter(filter)
        ),
        Value::HalfLifeTotalRoundedDown(filter) => format!(
            "half {} life total, rounded down",
            describe_possessive_player_filter(filter)
        ),
        Value::HalfStartingLifeTotalRoundedUp(filter) => format!(
            "half {} starting life total, rounded up",
            describe_possessive_player_filter(filter)
        ),
        Value::HalfStartingLifeTotalRoundedDown(filter) => format!(
            "half {} starting life total, rounded down",
            describe_possessive_player_filter(filter)
        ),
        Value::CardsInHand(filter) => format!(
            "the number of cards in {} hand",
            describe_possessive_player_filter(filter)
        ),
        Value::LifeGainedThisTurn(filter) => match filter {
            PlayerFilter::You => "the amount of life you gained this turn".to_string(),
            PlayerFilter::Opponent => {
                "the amount of life your opponents gained this turn".to_string()
            }
            _ => format!(
                "the amount of life {} gained this turn",
                describe_player_filter(filter)
            ),
        },
        Value::LifeLostThisTurn(filter) => match filter {
            PlayerFilter::You => "the total life you lost this turn".to_string(),
            PlayerFilter::Opponent => {
                "the total life your opponents lost this turn".to_string()
            }
            _ => format!(
                "the total life {} lost this turn",
                describe_player_filter(filter)
            ),
        },
        Value::NoncombatDamageDealtToPlayersThisTurn(filter) => match filter {
            PlayerFilter::You => {
                "the total amount of noncombat damage dealt to you this turn".to_string()
            }
            PlayerFilter::Opponent => {
                "the total amount of noncombat damage dealt to your opponents this turn".to_string()
            }
            _ => format!(
                "the total amount of noncombat damage dealt to {} this turn",
                describe_player_filter(filter)
            ),
        },
        Value::MaxCardsDrawnThisTurn(filter) => match filter {
            PlayerFilter::You => "the greatest number of cards you've drawn this turn".to_string(),
            PlayerFilter::Opponent => {
                "the greatest number of cards an opponent has drawn this turn".to_string()
            }
            PlayerFilter::Any => "the greatest number of cards a player has drawn this turn".to_string(),
            _ => format!(
                "the greatest number of cards {} has drawn this turn",
                describe_player_filter(filter)
            ),
        },
        Value::MaxCardsInHand(filter) => {
            // Prefer the oracle-style phrasing used on Adamaro, First to Desire.
            // (We keep this structured so that other filters still render coherently.)
            match filter {
                PlayerFilter::You => "the number of cards in your hand".to_string(),
                PlayerFilter::Opponent => "the number of cards in the hand of the opponent with the most cards in hand".to_string(),
                PlayerFilter::Any => "the number of cards in the hand of the player with the most cards in hand".to_string(),
                PlayerFilter::NotYou => "the number of cards in the hand of the player other than you with the most cards in hand".to_string(),
                _ => format!(
                    "the number of cards in the hand of the {} with the most cards in hand",
                    strip_leading_article(&describe_player_filter(filter))
                ),
            }
        }
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
        Value::SpellsCastThisTurnMatching {
            player,
            filter,
            exclude_source,
        } => {
            let base = pluralize_noun_phrase(&describe_for_each_filter(filter));
            let mut out = format!(
                "the number of {base} cast this turn by {}",
                describe_player_filter(player)
            );
            if *exclude_source {
                out.push_str(" other than this spell");
            }
            out
        }
        Value::CardTypesInGraveyard(filter) => format!(
            "the number of distinct card types in {} graveyard",
            describe_possessive_player_filter(filter)
        ),
        Value::Devotion { player, color } => format!(
            "{} devotion to {}",
            describe_possessive_player_filter(player),
            color.name().to_string()
        ),
        Value::ColorsOfManaSpentToCastThisSpell => {
            "the number of colors of mana spent to cast this spell".to_string()
        }
        Value::MagicGamesLostToOpponentsSinceLastWin => {
            "the number of Magic games you've lost to one of your opponents since you last won a game against them".to_string()
        }
        Value::EffectValue(_) => "X".to_string(),
        Value::EffectValueOffset(_, offset) => {
            if *offset == 0 {
                "X".to_string()
            } else if *offset > 0 {
                format!("X plus {}", offset)
            } else {
                format!("X minus {}", -offset)
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
            counter_type.description()
        ),
        Value::CountersOn(spec, Some(counter_type)) => format!(
            "the number of {} counter(s) on {}",
            counter_type.description(),
            describe_choose_spec(spec)
        ),
        Value::CountersOn(spec, None) => {
            format!("the number of counters on {}", describe_choose_spec(spec))
        }
        Value::TaggedCount => "the tagged object count".to_string(),
    }
}

pub(super) fn party_size_multiplier(value: &Value) -> Option<(PlayerFilter, i32)> {
    match value {
        Value::PartySize(filter) => Some((filter.clone(), 1)),
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

pub(super) fn spells_cast_this_turn_multiplier(value: &Value) -> Option<(PlayerFilter, i32)> {
    match value {
        Value::SpellsCastThisTurn(filter) => Some((filter.clone(), 1)),
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

pub(super) fn describe_spells_cast_this_turn_each(filter: &PlayerFilter) -> String {
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

pub(super) fn describe_signed_value(value: &Value) -> String {
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
        Value::EffectValue(_) => "+X".to_string(),
        Value::EffectValueOffset(_, offset) if *offset == 0 => "+X".to_string(),
        Value::EffectValueOffset(_, offset) if *offset > 0 => format!("+X plus {offset}"),
        Value::EffectValueOffset(_, offset) => format!("+X minus {}", -offset),
        Value::Fixed(n) => n.to_string(),
        _ => describe_value(value),
    }
}

pub(super) fn describe_toughness_delta_with_power_context(
    power: &Value,
    toughness: &Value,
) -> String {
    if matches!(power, Value::Fixed(n) if *n < 0) && matches!(toughness, Value::Fixed(0)) {
        "-0".to_string()
    } else {
        describe_signed_value(toughness)
    }
}

pub(super) fn describe_dynamic_runtime_pt_with_where_x(
    target: &str,
    plural_target: bool,
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

    let power_text = describe_value(power);
    let toughness_text = describe_value(toughness);
    let gets = if plural_target { "get" } else { "gets" };
    let power_text_lower = power_text.to_ascii_lowercase();
    let toughness_text_lower = toughness_text.to_ascii_lowercase();

    let power_is_count_like =
        !matches!(power, Value::Fixed(_)) && power_text_lower.contains("number of");
    let toughness_is_count_like =
        !matches!(toughness, Value::Fixed(_)) && toughness_text_lower.contains("number of");

    if power_is_count_like && toughness_is_count_like && power_text == toughness_text {
        return Some(format!(
            "{target} {gets} +X/+X {until_text}, where X is {power_text}"
        ));
    }
    if power_is_count_like && matches!(toughness, Value::Fixed(0)) {
        return Some(format!(
            "{target} {gets} +X/+0 {until_text}, where X is {power_text}"
        ));
    }
    if toughness_is_count_like && matches!(power, Value::Fixed(0)) {
        return Some(format!(
            "{target} {gets} +0/+X {until_text}, where X is {toughness_text}"
        ));
    }

    None
}

pub(super) fn describe_signed_i32(value: i32) -> String {
    if value >= 0 {
        format!("+{value}")
    } else {
        value.to_string()
    }
}

pub(super) fn choose_spec_is_plural(spec: &ChooseSpec) -> bool {
    effect_text_shared::choose_spec_is_plural(spec)
}

pub(super) fn choose_spec_allows_multiple(spec: &ChooseSpec) -> bool {
    match spec {
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

pub(super) fn owner_hand_phrase_for_spec(spec: &ChooseSpec) -> &'static str {
    if choose_spec_is_plural(spec) {
        "their owners' hands"
    } else {
        "its owner's hand"
    }
}

pub(super) fn owner_library_phrase_for_spec(spec: &ChooseSpec) -> &'static str {
    if choose_spec_is_plural(spec) {
        "their owners' libraries"
    } else {
        "its owner's library"
    }
}

pub(super) fn describe_put_counter_phrase(count: &Value, counter_type: CounterType) -> String {
    let counter_name = counter_type.description().into_owned();
    match count {
        Value::Fixed(1) => with_indefinite_article(&format!("{counter_name} counter")),
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

pub(super) fn describe_apply_continuous_target(
    effect: &crate::effects::ApplyContinuousEffect,
) -> (String, bool) {
    effect_text_shared::describe_apply_continuous_target(effect, describe_choose_spec, |filter| {
        pluralize_noun_phrase(&filter.description())
    })
}

pub(super) fn describe_apply_continuous_clauses(
    effect: &crate::effects::ApplyContinuousEffect,
    plural_target: bool,
) -> Vec<String> {
    let gets = if plural_target { "get" } else { "gets" };
    let has = if plural_target { "have" } else { "has" };
    let gains = if plural_target { "gain" } else { "gains" };
    let loses = if plural_target { "lose" } else { "loses" };

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
        crate::continuous::Modification::SwitchPowerToughness => {
            clauses.push("switches power and toughness".to_string());
        }
        crate::continuous::Modification::SetColors(colors) => {
            clauses.push(format!(
                "becomes {}",
                describe_token_color_words(*colors, false)
            ));
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
        crate::continuous::Modification::RemoveAbility(ability) => {
            if let Some(inline) = ability.granted_inline_ability() {
                clauses.push(format!("{loses} {}", describe_inline_ability(inline)));
            } else {
                clauses.push(format!("{loses} {}", ability.display()));
            }
        }
        crate::continuous::Modification::RemoveAllAbilities => {
            clauses.push(format!("{loses} all abilities"));
        }
        crate::continuous::Modification::AddAbilityGeneric(ability) => {
            clauses.push(format!("{gains} {}", describe_inline_ability(ability)));
        }
        crate::continuous::Modification::DoesntUntap => {
            clauses.push("can't untap".to_string());
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
            crate::effects::continuous::RuntimeModification::CopyOf(spec) => {
                clauses.push(format!("becomes a copy of {}", describe_choose_spec(spec)));
            }
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

pub(super) fn describe_apply_continuous_tail(
    effect: &crate::effects::ApplyContinuousEffect,
) -> Option<String> {
    if let Some(condition) = &effect.condition
        && matches!(effect.until, Until::ThisLeavesTheBattlefield)
    {
        return Some(format!(
            "while {}",
            lowercase_first(&describe_condition(condition))
        ));
    }
    if !matches!(effect.until, Until::Forever) {
        return Some(describe_until(&effect.until));
    }
    None
}

pub(super) fn describe_doesnt_untap_apply_continuous_effect(
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

    let mut text = if plural_target {
        format!("{target} don't untap during their controllers' untap steps")
    } else {
        format!("{target} doesn't untap during its controller's untap step")
    };
    if let Some(tail) = describe_apply_continuous_tail(effect) {
        text.push(' ');
        text.push_str(&tail);
    }
    Some(text)
}

pub(super) fn describe_apply_continuous_animation_effect(
    effect: &crate::effects::ApplyContinuousEffect,
    target: &str,
    plural_target: bool,
) -> Option<String> {
    let Some(crate::continuous::Modification::AddCardTypes(card_types)) = &effect.modification
    else {
        return None;
    };
    if !card_types.contains(&CardType::Creature) || !effect.runtime_modifications.is_empty() {
        return None;
    }

    let mut power = None;
    let mut toughness = None;
    let mut colors = None;
    let mut subtypes = Vec::new();
    let mut abilities = Vec::new();
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
            crate::continuous::Modification::AddAbility(ability) => {
                abilities.push(ability.clone());
            }
            _ => return None,
        }
    }

    let mut descriptor = Vec::new();
    if let Some(colors) = colors {
        descriptor.push(describe_token_color_words(colors, false));
    }
    if !subtypes.is_empty() {
        descriptor.push(
            subtypes
                .iter()
                .map(|subtype| subtype.to_string().to_ascii_lowercase())
                .collect::<Vec<_>>()
                .join(" "),
        );
    }
    let extra_card_types = card_types
        .iter()
        .copied()
        .filter(|card_type| *card_type != CardType::Creature)
        .map(|card_type| describe_card_type_word_local(card_type).to_string())
        .collect::<Vec<_>>();
    if !extra_card_types.is_empty() {
        descriptor.push(extra_card_types.join(" "));
    }
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
    let mut text = if let (Some(power), Some(toughness)) = (power, toughness) {
        let pt = format!("{}/{}", describe_value(power), describe_value(toughness));
        if plural_target {
            format!("{target} become {pt} {noun_phrase}")
        } else {
            format!("{target} becomes a {pt} {noun_phrase}")
        }
    } else if power.is_none() && toughness.is_none() {
        if plural_target {
            format!("{target} become {noun_phrase}")
        } else {
            format!("{target} becomes {}", with_indefinite_article(&noun_phrase))
        }
    } else {
        return None;
    };
    if !abilities.is_empty() {
        let ability_text = abilities
            .iter()
            .map(|ability| lowercase_first(&ability.display()))
            .collect::<Vec<_>>();
        text.push_str(" with ");
        text.push_str(&join_with_and(&ability_text));
    }
    if let Some(tail) = describe_apply_continuous_tail(effect) {
        text.push(' ');
        text.push_str(&tail);
    }
    Some(text)
}

pub(super) fn describe_apply_continuous_effect(
    effect: &crate::effects::ApplyContinuousEffect,
) -> Option<String> {
    let (target, plural_target) = describe_apply_continuous_target(effect);
    if let Some(text) = describe_apply_continuous_animation_effect(effect, &target, plural_target) {
        return Some(text);
    }
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

    let clauses = describe_apply_continuous_clauses(effect, plural_target);
    if clauses.is_empty() {
        return None;
    }

    let mut text = format!("{target} {}", join_with_and(&clauses));
    if let Some(tail) = describe_apply_continuous_tail(effect) {
        text.push(' ');
        text.push_str(&tail);
    }
    Some(text)
}

pub(super) fn describe_compact_apply_continuous_pair(
    first: &crate::effects::ApplyContinuousEffect,
    second: &crate::effects::ApplyContinuousEffect,
) -> Option<String> {
    if first.target != second.target
        || first.target_spec != second.target_spec
        || first.until != second.until
        || first.condition != second.condition
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
    if let Some(tail) = describe_apply_continuous_tail(first) {
        text.push(' ');
        text.push_str(&tail);
    }
    Some(text)
}

pub(super) fn choose_spec_references_tag(spec: &ChooseSpec, tag: &str) -> bool {
    match spec {
        ChooseSpec::Tagged(candidate) => candidate.as_str() == tag,
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            choose_spec_references_tag(inner, tag)
        }
        _ => false,
    }
}

pub(super) fn describe_attached_object_for_tag(tag: &str, spec: Option<&ChooseSpec>) -> String {
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

pub(super) fn describe_tag_attached_then_tap_or_untap(
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

pub(super) fn is_generated_internal_tag(tag: &str) -> bool {
    effect_text_shared::is_generated_internal_tag(tag)
}

pub(super) fn is_implicit_reference_tag(tag: &str) -> bool {
    effect_text_shared::is_implicit_reference_tag(tag)
}

pub(super) fn describe_until(until: &Until) -> String {
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

pub(super) fn describe_damage_filter(filter: &crate::prevention::DamageFilter) -> String {
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

    parts.join(" ")
}

pub(super) fn describe_prevention_target(
    target: &crate::prevention::PreventionTarget,
) -> &'static str {
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

pub(super) fn describe_restriction(restriction: &crate::effect::Restriction) -> String {
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
            format!("activated abilities of {} can't be activated", subject)
        }
        crate::effect::Restriction::ActivateTapAbilitiesOf(filter) => {
            let description = filter.description();
            let subject = description
                .strip_prefix("target ")
                .unwrap_or(description.as_str());
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
        crate::effect::Restriction::PreventDamage => "damage can't be prevented".to_string(),
        crate::effect::Restriction::Attack(filter) => {
            format!("{} can't attack", filter.description())
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
            format!(
                "{} must block {} if able",
                blockers.description(),
                attacker.description()
            )
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
            format!("{} can't be regenerated", filter.description())
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
        crate::effect::Restriction::BeTargetedPlayer(filter) => {
            format!("{} can't be targeted", describe_player_set_filter(filter))
        }
        crate::effect::Restriction::BeCountered(filter) => {
            format!("{} can't be countered", filter.description())
        }
        crate::effect::Restriction::Transform(filter) => {
            format!("{} can't transform", filter.description())
        }
        crate::effect::Restriction::AttackOrBlock(filter) => {
            format!("{} can't attack or block", filter.description())
        }
        crate::effect::Restriction::AttackOrBlockAlone(filter) => {
            format!("{} can't attack or block alone", filter.description())
        }
    }
}

pub(super) fn describe_comparison(cmp: &Comparison) -> String {
    match cmp {
        Comparison::GreaterThan(n) => format!("is greater than {n}"),
        Comparison::GreaterThanOrEqual(n) => format!("is at least {n}"),
        Comparison::Equal(n) => format!("is equal to {n}"),
        Comparison::LessThan(n) => format!("is less than {n}"),
        Comparison::LessThanOrEqual(n) => format!("is at most {n}"),
        Comparison::NotEqual(n) => format!("is not equal to {n}"),
    }
}

pub(super) fn basic_land_types_multiplier(value: &Value) -> Option<(&ObjectFilter, i32)> {
    match value {
        Value::BasicLandTypesAmong(filter) => Some((filter, 1)),
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

pub(super) fn describe_basic_land_type_scope(filter: &ObjectFilter) -> String {
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

pub(super) fn describe_basic_land_types_among(filter: &ObjectFilter) -> String {
    format!(
        "basic land type among {}",
        describe_basic_land_type_scope(filter)
    )
}

pub(super) fn describe_colors_among(filter: &ObjectFilter) -> String {
    format!("color among {}", describe_for_each_filter(filter))
}

pub(super) fn describe_effect_predicate(predicate: &EffectPredicate) -> String {
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

pub(super) fn tag_action_from_name(tag: &str) -> Option<&'static str> {
    let base = tag.split('_').next().unwrap_or(tag);
    match base {
        "sacrificed" => Some("sacrificed"),
        "destroyed" => Some("destroyed"),
        "exiled" => Some("exiled"),
        "discarded" => Some("discarded"),
        "died" => Some("died"),
        _ => None,
    }
}

pub(super) fn describe_player_tagged_object_text(tag: &TagKey, filter: &ObjectFilter) -> String {
    let card_context = tag.as_str().starts_with("discarded_")
        || tag.as_str().starts_with("exiled_")
        || tag.as_str().starts_with("revealed_");
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
        return with_indefinite_article(&format!("{} card", join_with_or(&words)));
    }

    let desc = filter.description();
    let stripped = strip_leading_article(&desc).to_ascii_lowercase();
    if card_context && stripped == "land" {
        return "a land card".to_string();
    }
    if card_context && stripped == "creature" {
        return "a creature card".to_string();
    }
    with_indefinite_article(&desc)
}

pub(super) fn is_owned_player_zone(zone: Option<Zone>) -> bool {
    matches!(
        zone,
        Some(Zone::Graveyard | Zone::Hand | Zone::Library | Zone::Command)
    )
}

pub(super) fn describe_owned_player_zone_filter(
    player: &PlayerFilter,
    filter: &ObjectFilter,
) -> String {
    let mut described = filter.clone();
    if described.owner.is_none() {
        described.owner = Some(player.clone());
    }
    described.description()
}

pub(super) fn describe_player_relative_condition(condition: &Condition) -> Option<String> {
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
        _ => None,
    }
}

pub(super) fn describe_condition(condition: &Condition) -> String {
    match condition {
        Condition::YouControl(filter) => format!("you control {}", filter.description()),
        Condition::OpponentControls(filter) => {
            format!("an opponent controls {}", filter.description())
        }
        Condition::PlayerControls { player, filter } => {
            let subject = describe_player_filter(player);
            if is_owned_player_zone(filter.zone) {
                let object_text = with_indefinite_article(&describe_owned_player_zone_filter(
                    player, filter,
                ));
                return format!(
                    "{} {} {}",
                    subject,
                    player_verb(&subject, "have", "has"),
                    object_text
                );
            }
            let mut described_filter = filter.clone();
            if described_filter
                .controller
                .as_ref()
                .is_some_and(|controller| controller == player)
            {
                described_filter.controller = None;
            }
            if matches!(
                described_filter.zone,
                Some(
                    Zone::Graveyard
                        | Zone::Hand
                        | Zone::Library
                        | Zone::Exile
                        | Zone::Command
                )
            ) {
                // For non-battlefield zones, the condition is about card presence rather than
                // "control". Prefer oracle-style existential phrasing.
                if described_filter.owner.is_none() {
                    described_filter.owner = Some(player.clone());
                }
                return format!("there is {}", described_filter.description());
            }
            let described = with_indefinite_article(strip_indefinite_article(&described_filter.description()));
            format!(
                "{} {} {}",
                subject,
                player_verb(&subject, "control", "controls"),
                described
            )
        }
        Condition::PlayerControlsAtLeast {
            player,
            filter,
            count,
        } => {
            let subject = describe_player_filter(player);
            if is_owned_player_zone(filter.zone) {
                let described =
                    strip_leading_article(&describe_owned_player_zone_filter(player, filter))
                        .to_string();
                let noun = pluralize_noun_phrase(&described);
                let count_text = small_number_word(*count)
                    .map(str::to_string)
                    .unwrap_or_else(|| count.to_string());
                return format!(
                    "{} {} {} or more {}",
                    subject,
                    player_verb(&subject, "have", "has"),
                    count_text,
                    noun
                );
            }
            let mut described_filter = filter.clone();
            if described_filter
                .controller
                .as_ref()
                .is_some_and(|controller| controller == player)
            {
                described_filter.controller = None;
            }
            let described = strip_indefinite_article(&described_filter.description()).to_string();
            let noun = pluralize_noun_phrase(&described);
            let count_text = small_number_word(*count)
                .map(str::to_string)
                .unwrap_or_else(|| count.to_string());
            format!(
                "{} {} {} or more {}",
                subject,
                player_verb(&subject, "control", "controls"),
                count_text,
                noun
            )
        }
        Condition::PlayerControlsExactly {
            player,
            filter,
            count,
        } => {
            let subject = describe_player_filter(player);
            if is_owned_player_zone(filter.zone) {
                let described =
                    strip_leading_article(&describe_owned_player_zone_filter(player, filter))
                        .to_string();
                let noun = if *count == 1 {
                    described
                } else {
                    pluralize_noun_phrase(&described)
                };
                let count_text = small_number_word(*count)
                    .map(str::to_string)
                    .unwrap_or_else(|| count.to_string());
                return format!(
                    "{} {} exactly {} {}",
                    subject,
                    player_verb(&subject, "have", "has"),
                    count_text,
                    noun
                );
            }
            let mut described_filter = filter.clone();
            if described_filter
                .controller
                .as_ref()
                .is_some_and(|controller| controller == player)
            {
                described_filter.controller = None;
            }
            let described = strip_indefinite_article(&described_filter.description()).to_string();
            let noun = if *count == 1 {
                described
            } else {
                pluralize_noun_phrase(&described)
            };
            let count_text = small_number_word(*count)
                .map(str::to_string)
                .unwrap_or_else(|| count.to_string());
            format!(
                "{} {} exactly {} {}",
                subject,
                player_verb(&subject, "control", "controls"),
                count_text,
                noun
            )
        }
        Condition::PlayerControlsAtLeastWithDifferentPowers {
            player,
            filter,
            count,
        } => {
            let subject = describe_player_filter(player);
            let mut described_filter = filter.clone();
            if described_filter
                .controller
                .as_ref()
                .is_some_and(|controller| controller == player)
            {
                described_filter.controller = None;
            }
            let described = strip_indefinite_article(&described_filter.description()).to_string();
            let noun = pluralize_noun_phrase(&described);
            let count_text = small_number_word(*count)
                .map(str::to_string)
                .unwrap_or_else(|| count.to_string());
            format!(
                "{} {} {} or more {} with different powers",
                subject,
                player_verb(&subject, "control", "controls"),
                count_text,
                noun
            )
        }
        Condition::PlayerControlsBasicLandTypesAmongLandsOrMore { player, count } => {
            let subject = describe_player_filter(player);
            let verb = player_verb(&subject, "control", "controls");
            let count_text = small_number_word(*count)
                .map(str::to_string)
                .unwrap_or_else(|| count.to_string());
            format!(
                "there are {} or more basic land types among lands {} {}",
                count_text, subject, verb
            )
        }
        Condition::PlayerHasCardTypesInGraveyardOrMore { player, count } => {
            let subject = describe_player_filter(player);
            let count_text = small_number_word(*count)
                .map(str::to_string)
                .unwrap_or_else(|| count.to_string());
            format!(
                "there are {} or more card types among cards in {} graveyard",
                count_text, subject
            )
        }
        Condition::PlayerControlsMost { player, filter } => {
            let controller = describe_player_filter(player);
            let mut described_filter = filter.clone();
            if described_filter
                .controller
                .as_ref()
                .is_some_and(|filter_controller| filter_controller == player)
            {
                described_filter.controller = None;
            }
            let mut subject = strip_indefinite_article(&described_filter.description()).to_string();
            if !subject.ends_with('s') {
                subject.push('s');
            }
            format!(
                "{} {} the most {}",
                controller,
                player_verb(&controller, "control", "controls"),
                subject
            )
        }
        Condition::PlayerControlsMoreThanYou { player, filter } => {
            let controller = describe_player_filter(player);
            let mut described_filter = filter.clone();
            if described_filter
                .controller
                .as_ref()
                .is_some_and(|filter_controller| filter_controller == player)
            {
                described_filter.controller = None;
            }
            let mut subject = strip_indefinite_article(&described_filter.description()).to_string();
            if !subject.ends_with('s') {
                subject.push('s');
            }
            format!(
                "{} {} more {} than you",
                controller,
                player_verb(&controller, "control", "controls"),
                subject
            )
        }
        Condition::PlayerLifeAtMostHalfStartingLifeTotal { player } => {
            let subject = if *player == PlayerFilter::You {
                "your".to_string()
            } else {
                format!("{}'s", describe_player_filter(player))
            };
            format!(
                "{subject} life total is less than or equal to half {} starting life total",
                describe_possessive_player_filter(player)
            )
        }
        Condition::PlayerLifeLessThanHalfStartingLifeTotal { player } => {
            let subject = if *player == PlayerFilter::You {
                "your".to_string()
            } else {
                format!("{}'s", describe_player_filter(player))
            };
            format!(
                "{subject} life total is less than half {} starting life total",
                describe_possessive_player_filter(player)
            )
        }
        Condition::PlayerHasLessLifeThanYou { player } => {
            format!("{} has less life than you", describe_player_filter(player))
        }
        Condition::PlayerHasMoreLifeThanYou { player } => {
            format!("{} has more life than you", describe_player_filter(player))
        }
        Condition::PlayerIsMonarch { player } => {
            format!("{} is the monarch", describe_player_filter(player))
        }
        Condition::PlayerHasCitysBlessing { player } => {
            format!("{} has the city's blessing", describe_player_filter(player))
        }
        Condition::LifeTotalOrLess(n) => format!("your life total is {n} or less"),
        Condition::LifeTotalOrGreater(n) => format!("your life total is {n} or greater"),
        Condition::CardsInHandOrMore(n) => format!("you have {n} or more cards in hand"),
        Condition::PlayerCardsInHandOrMore { player, count } => {
            format!("{} has {} or more cards in hand", describe_player_filter(player), count)
        }
        Condition::PlayerCardsInHandOrFewer { player, count } => {
            format!(
                "{} has {} or fewer cards in hand",
                describe_player_filter(player),
                count
            )
        }
        Condition::PlayerHasMoreCardsInHandThanYou { player } => {
            format!(
                "{} has more cards in hand than you",
                describe_player_filter(player)
            )
        }
        Condition::YouHaveCardInHandMatching(filter) => {
            let object_text = with_indefinite_article(&filter.description());
            format!("you have {object_text} in hand")
        }
        Condition::YourTurn => "it is your turn".to_string(),
        Condition::CreatureDiedThisTurn => "a creature died this turn".to_string(),
        Condition::CastSpellThisTurn => "a spell was cast this turn".to_string(),
        Condition::PlayerCastSpellsThisTurnOrMore { player, count } => {
            let subject = describe_player_filter(player);
            let count_text = small_number_word(*count)
                .map(str::to_string)
                .unwrap_or_else(|| count.to_string());
            format!(
                "{} {} cast {} or more spells this turn",
                subject,
                player_verb(&subject, "have", "has"),
                count_text
            )
        }
        Condition::AttackedThisTurn => "you attacked this turn".to_string(),
        Condition::OpponentLostLifeThisTurn => "an opponent lost life this turn".to_string(),
        Condition::PermanentLeftBattlefieldUnderYourControlThisTurn => {
            "a permanent left the battlefield under your control this turn".to_string()
        }
        Condition::SourceWasCast => "you cast it".to_string(),
        Condition::PlayerTappedLandForManaThisTurn { player } => {
            format!(
                "{} tapped a land for mana this turn",
                describe_player_filter(player)
            )
        }
        Condition::PlayerHadLandEnterBattlefieldThisTurn { player } => {
            format!(
                "{} had a land enter the battlefield under {} control this turn",
                describe_player_filter(player),
                describe_possessive_player_filter(player)
            )
        }
        Condition::NoSpellsWereCastLastTurn => "no spells were cast last turn".to_string(),
        Condition::SpellsWereCastLastTurnOrMore(count) => {
            let count_text = small_number_word(*count)
                .map(str::to_string)
                .unwrap_or_else(|| count.to_string());
            format!("{count_text} or more spells were cast last turn")
        }
        Condition::TargetIsTapped => "the target is tapped".to_string(),
        Condition::TargetIsBlocked => "the target is blocked".to_string(),
        Condition::TargetWasKicked => "the target spell was kicked".to_string(),
        Condition::ThisSpellWasKicked => "this spell was kicked".to_string(),
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
        Condition::EnchantedPermanentAttackedThisTurn => {
            "enchanted creature attacked this turn".to_string()
        }
        Condition::SourceIsTapped => "this source is tapped".to_string(),
        Condition::SourceIsSaddled => "this source is saddled".to_string(),
        Condition::SourceIsFaceDown => "this source is transformed".to_string(),
        Condition::SourceHasNoCounter(counter_type) => format!(
            "there are no {} counters on this source",
            counter_type.description()
        ),
        Condition::SourceHasCounterAtLeast { counter_type, count } => format!(
            "there are {count} or more {} counters on this source",
            counter_type.description()
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
        Condition::ColorsOfManaSpentToCastThisSpellOrMore(amount) => {
            let amount_text = small_number_word(*amount)
                .map(str::to_string)
                .unwrap_or_else(|| amount.to_string());
            format!("{} or more colors of mana were spent to cast this spell", amount_text)
        }
        Condition::YouControlCommander => "you control your commander".to_string(),
        Condition::TargetMatches(filter) => {
            let desc = filter.description();
            let stripped = strip_leading_article(&desc).to_ascii_lowercase();
            if stripped == "land" {
                "it's a land card".to_string()
            } else if stripped == "creature" {
                "it's a creature".to_string()
            } else {
                format!("the target matches {desc}")
            }
        }
        Condition::TargetIsSoulbondPaired => {
            "the target is paired with another creature".to_string()
        }
        Condition::TaggedObjectMatches(tag, filter) => {
            let desc = filter.description();
            if is_implicit_reference_tag(tag.as_str()) {
                // Keep implicit tags oracle-like: use pronouns rather than exposing tag keys.
                let subject = if matches!(tag.as_str(), "triggering" | "damaged") {
                    "that object"
	                } else {
	                    "it"
	                };
	                let card_context = is_generated_internal_tag(tag.as_str())
	                    || tag.as_str().starts_with("exiled_")
	                    || tag.as_str().starts_with("revealed_");
	                let is_clause = |noun_phrase: &str| {
	                    let phrase = with_indefinite_article(noun_phrase);
	                    if subject == "it" {
	                        format!("it's {phrase}")
	                    } else {
	                        format!("{subject} is {phrase}")
	                    }
	                };

	                if card_context
	                    && !filter.card_types.is_empty()
	                    && filter.zone.is_none()
	                    && filter.controller.is_none()
	                    && filter.owner.is_none()
	                    && !filter.single_graveyard
	                    && filter.targets_player.is_none()
	                    && filter.targets_object.is_none()
	                    && !filter.targets_any_of
	                    && filter.all_card_types.is_empty()
	                    && filter.excluded_card_types.is_empty()
	                    && filter.subtypes.is_empty()
	                    && !filter.type_or_subtype_union
	                    && filter.excluded_subtypes.is_empty()
	                    && filter.supertypes.is_empty()
	                    && filter.excluded_supertypes.is_empty()
	                    && filter.colors.is_none()
	                    && filter.excluded_colors.is_empty()
	                    && !filter.colorless
	                    && !filter.multicolored
	                    && !filter.monocolored
	                    && filter.all_colors.is_none()
	                    && filter.exactly_two_colors.is_none()
	                    && !filter.historic
	                    && !filter.nonhistoric
	                    && !filter.token
	                    && !filter.nontoken
	                    && filter.face_down.is_none()
	                    && !filter.other
	                    && !filter.tapped
	                    && !filter.untapped
	                    && !filter.attacking
	                    && !filter.nonattacking
	                    && !filter.blocking
	                    && !filter.nonblocking
	                    && !filter.blocked
	                    && !filter.unblocked
	                    && !filter.entered_since_your_last_turn_ended
	                    && filter.power.is_none()
	                    && filter.toughness.is_none()
	                    && filter.mana_value.is_none()
	                    && filter.mana_value_eq_counters_on_source.is_none()
	                    && !filter.has_mana_cost
	                    && !filter.has_tap_activated_ability
	                    && !filter.no_abilities
	                    && !filter.no_x_in_cost
	                    && filter.with_counter.is_none()
	                    && filter.without_counter.is_none()
	                    && filter.name.is_none()
	                    && filter.excluded_name.is_none()
	                    && filter.alternative_cast.is_none()
	                    && filter.static_abilities.is_empty()
	                    && filter.excluded_static_abilities.is_empty()
	                    && filter.ability_markers.is_empty()
	                    && filter.excluded_ability_markers.is_empty()
	                    && !filter.is_commander
	                    && !filter.noncommander
	                    && filter.tagged_constraints.is_empty()
	                    && filter.specific.is_none()
	                    && filter.any_of.is_empty()
	                    && !filter.source
	                {
	                    let words = filter
	                        .card_types
	                        .iter()
	                        .map(|card_type| describe_card_type_word_local(*card_type).to_string())
	                        .collect::<Vec<_>>();
	                    let noun_phrase = format!("{} card", join_with_or(&words));
	                    return is_clause(&noun_phrase);
	                }

	                let stripped = strip_leading_article(&desc).to_ascii_lowercase();
	                if stripped == "land" {
	                    let noun = if card_context { "land card" } else { "land" };
	                    return is_clause(noun);
	                }
	                if stripped == "creature" {
	                    let noun = if card_context {
	                        "creature card"
	                    } else {
	                        "creature"
	                    };
	                    return is_clause(noun);
	                }
	                return format!("{subject} matches {desc}");
	            }
                format!("the tagged object '{}' matches {desc}", tag.as_str())
            }
        Condition::TaggedObjectIsSoulbondPaired(tag) => {
            if is_implicit_reference_tag(tag.as_str()) {
                "it's paired with another creature".to_string()
            } else {
                format!(
                    "the tagged object '{}' is paired with another creature",
                    tag.as_str()
                )
            }
        }
        Condition::PlayerTaggedObjectMatches { player, tag, filter } => {
            if let Some(action) = tag_action_from_name(tag.as_str()) {
                let object_text = describe_player_tagged_object_text(tag, filter);
                format!(
                    "{} {} {} this way",
                    describe_player_filter(player),
                    action,
                    object_text
                )
            } else {
                format!(
                    "{} had the tagged object '{}' matching {}",
                    describe_player_filter(player),
                    tag.as_str(),
                    filter.description()
                )
            }
        }
        Condition::PlayerTaggedObjectEnteredBattlefieldThisTurn { player, tag } => {
            if let Some(action) = tag_action_from_name(tag.as_str()) {
                format!("{} {} it this way", describe_player_filter(player), action)
            } else {
                format!(
                    "{} had the tagged object '{}' enter the battlefield under their control this turn",
                    describe_player_filter(player),
                    tag.as_str()
                )
            }
        }
        Condition::PlayerOwnsCardNamedInZones { player, name, zones } => {
            let subject = describe_player_filter(player);
            let possessive = describe_possessive_player_filter(player);
            let mut zone_phrases = Vec::new();
            for zone in zones {
                match zone {
                    Zone::Exile => zone_phrases.push("in exile".to_string()),
                    Zone::Hand => zone_phrases.push(format!("in {possessive} hand")),
                    Zone::Graveyard => zone_phrases.push(format!("in {possessive} graveyard")),
                    Zone::Library => zone_phrases.push(format!("in {possessive} library")),
                    Zone::Battlefield => zone_phrases.push("on the battlefield".to_string()),
                    Zone::Stack => zone_phrases.push("on the stack".to_string()),
                    Zone::Command => zone_phrases.push("in the command zone".to_string()),
                }
            }
            let zones_text = join_with_and(&zone_phrases);
            format!(
                "{} {} a card named {} {}",
                subject,
                player_verb(&subject, "own", "owns"),
                name,
                zones_text
            )
        }
        Condition::FirstTimeThisTurn => "this is the first time this ability triggered this turn"
            .to_string(),
        Condition::MaxTimesEachTurn(limit) => {
            format!("this ability has triggered fewer than {limit} times this turn")
        }
        Condition::TriggeringObjectWasEnchanted => "the triggering object was enchanted".to_string(),
        Condition::TriggeringObjectHadCounters {
            counter_type,
            min_count,
        } => format!(
            "the triggering object had {min_count} or more {} counters",
            counter_type.description()
        ),
        Condition::ControlCreaturesTotalPowerAtLeast(power) => format!(
            "creatures you control have total power {power} or greater"
        ),
        Condition::CardInYourGraveyard { card_types, subtypes } => {
            if card_types.is_empty() && subtypes.is_empty() {
                "there is a card in your graveyard".to_string()
            } else if subtypes.is_empty() {
                let types = card_types
                    .iter()
                    .map(|t| format!("{t:?}").to_ascii_lowercase())
                    .collect::<Vec<_>>();
                format!("there is a {} card in your graveyard", join_with_or(&types))
            } else if card_types.is_empty() {
                let types = subtypes
                    .iter()
                    .map(|t| format!("{t:?}").to_ascii_lowercase())
                    .collect::<Vec<_>>();
                format!("there is an {} card in your graveyard", join_with_or(&types))
            } else {
                let card_types = card_types
                    .iter()
                    .map(|t| format!("{t:?}").to_ascii_lowercase())
                    .collect::<Vec<_>>();
                let subtypes = subtypes
                    .iter()
                    .map(|t| format!("{t:?}").to_ascii_lowercase())
                    .collect::<Vec<_>>();
                format!(
                    "there is a {} {} card in your graveyard",
                    join_with_or(&subtypes),
                    join_with_or(&card_types)
                )
            }
        }
        Condition::ActivationTiming(timing) => {
            let label = match timing {
                crate::ability::ActivationTiming::AnyTime => "any time",
                crate::ability::ActivationTiming::SorcerySpeed => "sorcery speed",
                crate::ability::ActivationTiming::DuringCombat => "during combat",
                crate::ability::ActivationTiming::OncePerTurn => "once per turn",
                crate::ability::ActivationTiming::DuringYourTurn => "during your turn",
                crate::ability::ActivationTiming::DuringOpponentsTurn => "during opponents' turns",
            };
            format!("timing restriction: {label}")
        }
        Condition::MaxActivationsPerTurn(limit) => {
            format!("this ability has been activated fewer than {limit} times this turn")
        }
        Condition::SourceIsEquipped => "this permanent is equipped".to_string(),
        Condition::SourceIsEnchanted => "this permanent is enchanted".to_string(),
        Condition::EnchantedPermanentIsCreature => {
            "enchanted permanent is a creature".to_string()
        }
        Condition::EnchantedPermanentIsEquipment => {
            "enchanted permanent is an equipment".to_string()
        }
        Condition::EnchantedPermanentIsVehicle => {
            "enchanted permanent is a vehicle".to_string()
        }
        Condition::EquippedCreatureTapped => "equipped creature is tapped".to_string(),
        Condition::EquippedCreatureUntapped => "equipped creature is untapped".to_string(),
        Condition::CountComparison { display, .. } => display
            .as_ref()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "count comparison".to_string()),
        Condition::OwnsCardExiledWithCounter(counter) => format!(
            "you own a card in exile with a {} counter on it",
            counter.description()
        ),
        Condition::SourceAttackedThisTurn => "this creature attacked this turn".to_string(),
        Condition::SourceIsUntapped => "this source is untapped".to_string(),
        Condition::SourceIsAttacking => "this source is attacking".to_string(),
        Condition::SourceIsBlocking => "this source is blocking".to_string(),
        Condition::SourceIsSoulbondPaired => {
            "this creature is paired with another creature".to_string()
        }
        Condition::PlayerGraveyardHasCardsAtLeast { player, count } => {
            format!("{player:?}'s graveyard has {count} or more cards")
        }
        Condition::XValueAtLeast(min) => format!("X is {min} or more"),
        Condition::Custom(id) => format!("custom condition {id}"),
        Condition::Unmodeled(text) => text.clone(),
        Condition::Not(inner) => {
            if let Condition::TargetSpellManaSpentToCastAtLeast {
                amount: 1,
                symbol: None,
            } = inner.as_ref()
            {
                "no mana was spent to cast the target spell".to_string()
            } else if let Condition::CardsInHandOrMore(1) = inner.as_ref() {
                "you have no cards in hand".to_string()
            } else if let Condition::PlayerControls { player, filter } = inner.as_ref() {
                let subject = describe_player_filter(player);
                let mut described_filter = filter.clone();
                if described_filter
                    .controller
                    .as_ref()
                    .is_some_and(|controller| controller == player)
                {
                    described_filter.controller = None;
                }
                let described = described_filter.description();
                let mut object_text = strip_indefinite_article(&described).to_string();
                if let Some(rest) = object_text.strip_prefix("another ") {
                    // "You control no other permanents" is substantially closer to oracle text than
                    // the ungrammatical "You control no another permanent".
                    object_text = format!("other {}", pluralize_noun_phrase(rest));
                }
                let references_tagged_object =
                    described_filter.tagged_constraints.iter().any(|constraint| {
                        matches!(
                            constraint.relation,
                            crate::filter::TaggedOpbjectRelation::IsTaggedObject
                        )
                    });
                if references_tagged_object {
                    return format!(
                        "{} {} neither {}",
                        subject,
                        player_verb(&subject, "control", "controls"),
                        object_text
                    );
                }
                format!(
                    "{} {} no {}",
                    subject,
                    player_verb(&subject, "control", "controls"),
                    object_text
                )
            } else {
                format!("not ({})", describe_condition(inner))
            }
        }
        Condition::And(left, right) => {
            // Avoid parentheses here: the semantic comparison pipeline strips parentheticals,
            // and these are just internal grouping markers, not oracle reminder text.
            format!("{} and {}", describe_condition(left), describe_condition(right))
        }
        Condition::Or(left, right) => {
            format!("{} or {}", describe_condition(left), describe_condition(right))
        }
    }
}
