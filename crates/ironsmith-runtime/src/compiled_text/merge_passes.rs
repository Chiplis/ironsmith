use super::*;
use crate::filter::ObjectFilterExt as _;

pub(super) fn strip_render_heading(line: &str) -> String {
    let Some((prefix, rest)) = line.split_once(':') else {
        return line.trim().to_string();
    };
    if is_render_heading_prefix(prefix) {
        rest.trim().to_string()
    } else {
        line.trim().to_string()
    }
}

pub(super) fn is_keyword_phrase(phrase: &str) -> bool {
    let lower = phrase.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return false;
    }
    if is_landwalk_keyword_phrase(&lower) {
        return true;
    }
    if lower.starts_with("protection from ") {
        return true;
    }
    if lower.starts_with("hexproof from ") {
        return true;
    }
    if lower.starts_with("partner with ")
        || lower.starts_with("partner-")
        || lower.starts_with("partner\u{2013}")
        || lower.starts_with("partner\u{2014}")
    {
        return true;
    }
    if lower.starts_with("ward ") {
        return true;
    }
    if lower == "sunburst"
        || lower.starts_with("bushido ")
        || lower.starts_with("fading ")
        || lower.starts_with("fabricate ")
        || lower.starts_with("graft ")
        || lower.starts_with("modular ")
        || lower.starts_with("rampage ")
        || lower.starts_with("scavenge ")
        || lower.starts_with("transmute ")
        || lower.starts_with("toxic ")
        || lower.starts_with("vanishing ")
    {
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
            | "skulk"
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
            | "battle cry"
            | "daybound"
            | "dethrone"
            | "enlist"
            | "extort"
            | "evolve"
            | "ingest"
            | "melee"
            | "myriad"
            | "nightbound"
            | "prowess"
            | "provoke"
            | "riot"
            | "training"
            | "station"
            | "persist"
            | "undying"
            | "partner"
            | "assist"
    )
}

fn is_landwalk_keyword_phrase(lower: &str) -> bool {
    if matches!(
        lower,
        "landwalk" | "nonbasic landwalk" | "artifact landwalk" | "legendary landwalk"
    ) {
        return true;
    }

    if let Some(rest) = lower.strip_prefix("snow ") {
        return is_landwalk_subtype_compound(rest);
    }

    is_landwalk_subtype_compound(lower)
}

fn is_landwalk_subtype_compound(lower: &str) -> bool {
    matches!(
        lower,
        "plainswalk" | "islandwalk" | "swampwalk" | "mountainwalk" | "forestwalk" | "desertwalk"
    )
}

pub(super) fn split_have_clause(clause: &str) -> Option<(String, String)> {
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

pub(super) fn split_lose_all_abilities_clause(clause: &str) -> Option<String> {
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

pub(super) fn extract_base_pt_tail_for_subject(line: &str, subject: &str) -> Option<String> {
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

pub(super) fn normalize_global_subject_number(subject: &str) -> String {
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

pub(super) fn subject_is_plural(subject: &str) -> bool {
    let lower = subject.trim().to_ascii_lowercase();
    lower.starts_with("all ")
        || lower.starts_with("other ")
        || lower.starts_with("each ")
        || lower.starts_with("those ")
        || lower.ends_with('s')
}

pub(super) fn split_subject_predicate_clause(line: &str) -> Option<(&str, &str, &str)> {
    let mut best: Option<(usize, &str)> = None;
    for verb in [
        " gets ",
        " get ",
        " has ",
        " have ",
        " gains ",
        " gain ",
        " is ",
        " are ",
        " can't be ",
    ] {
        if let Some(idx) = line.find(verb)
            && best.is_none_or(|(best_idx, _)| idx < best_idx)
        {
            best = Some((idx, verb));
        }
    }
    let (idx, verb) = best?;
    let subject = line[..idx].trim();
    let rest = line[idx + verb.len()..].trim();
    if !subject.is_empty() && !rest.is_empty() {
        Some((subject, verb.trim(), rest))
    } else {
        None
    }
}

#[derive(Debug, Clone)]
struct ConditionalSubjectPredicate {
    condition: String,
    subject: String,
    verb: String,
    predicate: String,
}

fn parse_conditional_subject_predicate(line: &str) -> Option<ConditionalSubjectPredicate> {
    let trimmed = line.trim().trim_end_matches('.');
    if trimmed.is_empty() {
        return None;
    }

    if let Some((condition, body)) = trimmed.split_once(", ") {
        let condition = condition.trim();
        let normalized_condition = if condition.eq_ignore_ascii_case("During your turn") {
            "As long as it's your turn".to_string()
        } else if condition.to_ascii_lowercase().starts_with("as long as ") {
            condition.to_string()
        } else {
            String::new()
        };
        if !normalized_condition.is_empty() {
            let (subject, verb, predicate) = split_subject_predicate_clause(body)?;
            return Some(ConditionalSubjectPredicate {
                condition: normalized_condition,
                subject: subject.trim().to_string(),
                verb: verb.trim().to_string(),
                predicate: predicate.trim().to_string(),
            });
        }
    }

    let (subject, verb, predicate_with_condition) = split_subject_predicate_clause(trimmed)?;
    let (predicate, condition) = predicate_with_condition.rsplit_once(" as long as ")?;
    Some(ConditionalSubjectPredicate {
        condition: format!("As long as {}", condition.trim()),
        subject: subject.trim().to_string(),
        verb: verb.trim().to_string(),
        predicate: predicate.trim().to_string(),
    })
}

fn is_creature_addition_predicate(predicate: &str) -> bool {
    let lower = predicate.trim().to_ascii_lowercase();
    lower == "a creature in addition to its other types"
        || lower == "creature in addition to its other types"
        || lower == "creatures in addition to their other types"
}

fn subtype_addition_predicate(predicate: &str) -> Option<String> {
    let trimmed = predicate.trim();
    let lower = trimmed.to_ascii_lowercase();
    for suffix in [
        " in addition to its other types",
        " in addition to their other types",
    ] {
        if lower.ends_with(suffix) {
            let subtype = trimmed[..trimmed.len() - suffix.len()].trim();
            if subtype.is_empty() || is_creature_addition_predicate(trimmed) {
                return None;
            }
            return Some(singularize_terminal_subject_word(subtype));
        }
    }
    None
}

#[derive(Debug, Clone)]
struct TypeAdditionLine {
    subject: String,
    verb: String,
    type_phrase: String,
}

fn parse_type_addition_line(line: &str) -> Option<TypeAdditionLine> {
    let trimmed = line.trim().trim_end_matches('.');
    let (subject, verb, predicate) = split_subject_predicate_clause(trimmed)?;
    if !matches!(verb, "is" | "are") {
        return None;
    }
    let type_phrase = subtype_addition_predicate(predicate)?;
    Some(TypeAdditionLine {
        subject: subject.trim().to_string(),
        verb: verb.trim().to_string(),
        type_phrase,
    })
}

pub(super) fn merge_same_true_type_addition_lines(lines: Vec<String>) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;

    while idx < lines.len() {
        let Some(first) = parse_type_addition_line(&lines[idx]) else {
            merged.push(lines[idx].clone());
            idx += 1;
            continue;
        };

        let mut same_true_subjects = Vec::new();
        let mut consumed = 1usize;
        while idx + consumed < lines.len() {
            let Some(next) = parse_type_addition_line(&lines[idx + consumed]) else {
                break;
            };
            if !first.type_phrase.eq_ignore_ascii_case(&next.type_phrase)
                || !first.verb.eq_ignore_ascii_case(&next.verb)
            {
                break;
            }
            same_true_subjects.push(lowercase_first(&next.subject));
            consumed += 1;
        }

        if same_true_subjects.len() < 2 {
            merged.push(lines[idx].clone());
            idx += 1;
            continue;
        }

        merged.push(format!(
            "{} The same is true for {}.",
            lines[idx].trim(),
            join_with_and(&same_true_subjects)
        ));
        idx += consumed;
    }

    merged
}

#[derive(Debug, Clone)]
struct ColorLine {
    subject: String,
    verb: String,
    color: String,
}

fn parse_color_line(line: &str) -> Option<ColorLine> {
    let trimmed = line.trim().trim_end_matches('.');
    let (subject, verb, predicate) = split_subject_predicate_clause(trimmed)?;
    if !matches!(verb, "is" | "are") || !is_color_predicate(predicate) {
        return None;
    }

    Some(ColorLine {
        subject: normalize_nonbattlefield_owned_cards_subject(subject.trim())
            .unwrap_or_else(|| subject.trim().to_string()),
        verb: verb.trim().to_string(),
        color: predicate.trim().to_string(),
    })
}

fn is_color_predicate(predicate: &str) -> bool {
    matches!(
        predicate.trim().to_ascii_lowercase().as_str(),
        "white" | "blue" | "black" | "red" | "green" | "colorless"
    )
}

fn normalize_nonbattlefield_owned_cards_subject(subject: &str) -> Option<String> {
    let lower = subject.to_ascii_lowercase();
    let parts = lower.split(" or ").collect::<Vec<_>>();
    if parts.len() != 5 {
        return None;
    }

    let zones = ["hand", "library", "graveyard", "exile", "command zone"];
    let mut prefix: Option<&str> = None;
    for (part, zone) in parts.iter().zip(zones) {
        let suffix = format!(" cards in your {zone}");
        let current_prefix = part.strip_suffix(&suffix)?.trim();
        if current_prefix.is_empty() {
            return None;
        }
        match prefix {
            Some(existing) if existing != current_prefix => return None,
            Some(_) => {}
            None => prefix = Some(current_prefix),
        }
    }

    Some(format!(
        "{} cards you own that aren't on the battlefield",
        prefix?
    ))
}

pub(super) fn merge_same_true_color_lines(lines: Vec<String>) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;

    while idx < lines.len() {
        let Some(first) = parse_color_line(&lines[idx]) else {
            merged.push(lines[idx].clone());
            idx += 1;
            continue;
        };

        let mut same_true_subjects = Vec::new();
        let mut consumed = 1usize;
        while idx + consumed < lines.len() {
            let Some(next) = parse_color_line(&lines[idx + consumed]) else {
                break;
            };
            if !first.color.eq_ignore_ascii_case(&next.color)
                || !first.verb.eq_ignore_ascii_case(&next.verb)
            {
                break;
            }
            same_true_subjects.push(lowercase_first(&next.subject));
            consumed += 1;
        }

        if same_true_subjects.len() < 2 {
            merged.push(lines[idx].clone());
            idx += 1;
            continue;
        }

        merged.push(format!(
            "{} The same is true for {}.",
            lines[idx].trim(),
            join_with_and(&same_true_subjects)
        ));
        idx += consumed;
    }

    merged
}

fn indefinite_article_for_phrase(phrase: &str) -> &'static str {
    match phrase.chars().next().map(|ch| ch.to_ascii_lowercase()) {
        Some('a' | 'e' | 'i' | 'o' | 'u') => "an",
        _ => "a",
    }
}

fn singularize_terminal_subject_word(phrase: &str) -> String {
    if let Some((head, tail)) = phrase.rsplit_once(' ') {
        let singular = singularize_subject_word(tail);
        if head.trim().is_empty() {
            singular
        } else {
            format!("{head} {singular}")
        }
    } else {
        singularize_subject_word(phrase)
    }
}

fn singularize_subject_word(word: &str) -> String {
    let lower = word.to_ascii_lowercase();
    let preserve_cap = word
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase());
    let make_case = |singular: &str| {
        if preserve_cap {
            capitalize_first(singular)
        } else {
            singular.to_string()
        }
    };

    if lower == "mice" {
        return make_case("mouse");
    }
    if lower == "elves" {
        return make_case("elf");
    }
    if lower == "dwarves" {
        return make_case("dwarf");
    }
    if lower.ends_with("ies") && word.len() > 3 {
        return format!("{}y", &word[..word.len() - 3]);
    }
    if lower.ends_with('s') && !lower.ends_with("ss") && word.len() > 1 {
        return word[..word.len() - 1].to_string();
    }
    word.to_string()
}

fn singularize_filter_subject(subject: &str) -> String {
    let mut singular = subject.trim().to_string();
    for (plural, singular_word) in [
        ("permanents", "permanent"),
        ("creatures", "creature"),
        ("artifacts", "artifact"),
        ("enchantments", "enchantment"),
        ("lands", "land"),
        ("planeswalkers", "planeswalker"),
        ("battles", "battle"),
        ("spells", "spell"),
        ("cards", "card"),
        ("tokens", "token"),
        ("abilities", "ability"),
        ("sources", "source"),
    ] {
        singular = replace_ascii_word_ci(&singular, plural, singular_word);
    }
    compact_repeated_mana_value_or_subject(&singular)
}

fn compact_repeated_mana_value_or_subject(subject: &str) -> String {
    let lower = subject.to_ascii_lowercase();
    for suffix in [
        " you control",
        " you don't control",
        " that player controls",
        " you own",
        " you don't own",
        " an opponent owns",
        " a player owns",
        " target player owns",
        " target opponent owns",
    ] {
        if !lower.ends_with(suffix) {
            continue;
        }
        let separator = format!("{suffix} or ");
        let Some(separator_idx) = lower.find(&separator) else {
            continue;
        };
        let left = &subject[..separator_idx + suffix.len()];
        let right = &subject[separator_idx + separator.len()..];
        let Some((left_base, left_value)) = split_mana_value_clause_with_suffix(left, suffix)
        else {
            continue;
        };
        let Some((right_base, right_value)) = split_mana_value_clause_with_suffix(right, suffix)
        else {
            continue;
        };
        if !left_value.eq_ignore_ascii_case(right_value) {
            continue;
        }
        return format!(
            "{} and {} {} with mana value {}",
            left_base.trim(),
            right_base.trim(),
            suffix.trim(),
            left_value.trim()
        );
    }
    subject.to_string()
}

fn split_mana_value_clause_with_suffix<'a>(
    clause: &'a str,
    suffix: &str,
) -> Option<(&'a str, &'a str)> {
    let (base, value_with_suffix) = clause.split_once(" with mana value ")?;
    let value = value_with_suffix.strip_suffix(suffix)?;
    Some((base.trim(), value.trim()))
}

fn replace_ascii_word_ci(input: &str, from: &str, to: &str) -> String {
    let lower = input.to_ascii_lowercase();
    let mut output = String::with_capacity(input.len());
    let mut search_start = 0usize;
    let mut copy_start = 0usize;

    while let Some(relative_idx) = lower[search_start..].find(from) {
        let idx = search_start + relative_idx;
        let end = idx + from.len();
        let before_ok = idx == 0
            || !input[..idx]
                .chars()
                .next_back()
                .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '\'');
        let after_ok = end >= input.len()
            || !input[end..]
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '\'');

        if before_ok && after_ok {
            output.push_str(&input[copy_start..idx]);
            output.push_str(to);
            copy_start = end;
        }
        search_start = end;
    }

    output.push_str(&input[copy_start..]);
    output
}

fn can_merge_conditional_state_bundle(
    left: &ConditionalSubjectPredicate,
    right: &ConditionalSubjectPredicate,
) -> bool {
    matches!(left.verb.as_str(), "is" | "are")
        && matches!(right.verb.as_str(), "is" | "are")
        && conditioned_subjects_equivalent(&left.subject, &right.subject)
        && left.condition.eq_ignore_ascii_case(&right.condition)
}

fn conditioned_subjects_equivalent(left: &str, right: &str) -> bool {
    conditioned_subject_key(left) == conditioned_subject_key(right)
}

fn conditioned_subject_key(subject: &str) -> String {
    let lower = subject.trim().to_ascii_lowercase();
    if lower == "this source" {
        return "this creature".to_string();
    }
    if let Some(rest) = lower.strip_prefix("each creature ") {
        return format!("creatures {rest}");
    }
    lower
}

pub(super) fn can_merge_subject_predicates(left_verb: &str, right_verb: &str) -> bool {
    let is_get = |verb: &str| matches!(verb, "gets" | "get");
    let is_trait = |verb: &str| matches!(verb, "has" | "have" | "gains" | "gain");
    let is_state = |verb: &str| matches!(verb, "is" | "are");
    let is_cant_be = |verb: &str| verb == "can't be";

    (is_get(left_verb) && is_trait(right_verb))
        || (is_trait(left_verb) && is_get(right_verb))
        || (is_trait(left_verb) && is_trait(right_verb))
        || (is_trait(left_verb) && is_state(right_verb))
        || (is_state(left_verb) && is_trait(right_verb))
        || ((left_verb == "gets" && right_verb == "is")
            || (left_verb == "is" && right_verb == "gets"))
        || (is_state(left_verb) && is_state(right_verb))
        || (is_cant_be(left_verb) && (is_get(right_verb) || is_trait(right_verb)))
        || (is_cant_be(right_verb) && (is_get(left_verb) || is_trait(left_verb)))
}

fn format_conditioned_subject_predicate_merge(
    left: &ConditionalSubjectPredicate,
    left_predicate: &str,
    right_verb: &str,
    right_predicate: &str,
) -> String {
    if left
        .condition
        .eq_ignore_ascii_case("As long as it's your turn")
    {
        let (subject, left_verb) = during_your_turn_subject_and_verb(&left.subject, &left.verb);
        let (_, right_verb) = during_your_turn_subject_and_verb(&left.subject, right_verb);
        return format!(
            "During your turn, {} {} {} and {} {}",
            subject, left_verb, left_predicate, right_verb, right_predicate
        );
    }

    let condition = left
        .condition
        .trim_start_matches("As long as ")
        .trim_start_matches("as long as ")
        .trim();
    let ability_word = if condition.eq_ignore_ascii_case("you have no cards in hand") {
        "Hellbent \u{2014} "
    } else {
        ""
    };
    let is_get = |verb: &str| matches!(verb, "gets" | "get");
    let is_trait = |verb: &str| matches!(verb, "has" | "have" | "gains" | "gain");
    let is_cant_be_blocked = |verb: &str, predicate: &str| {
        verb == "can't be" && predicate.eq_ignore_ascii_case("blocked")
    };
    if is_cant_be_blocked(right_verb, right_predicate) {
        return format!(
            "{ability_word}As long as {condition}, {} {} {} and can't be blocked",
            lowercase_first(&left.subject),
            left.verb,
            left_predicate,
        );
    }
    if is_cant_be_blocked(&left.verb, left_predicate) {
        return format!(
            "{ability_word}As long as {condition}, {} {right_verb} {right_predicate} and can't be blocked",
            lowercase_first(&left.subject),
        );
    }
    if is_get(&left.verb) && is_trait(right_verb) {
        return format!(
            "{ability_word}As long as {condition}, {} {} {} and {} {}",
            lowercase_first(&left.subject),
            left.verb,
            left_predicate,
            have_verb_for_subject(&left.subject),
            right_predicate
        );
    }
    if is_trait(&left.verb) && is_trait(right_verb) {
        return format!(
            "{ability_word}As long as {condition}, {} {} {} and {}",
            lowercase_first(&left.subject),
            have_verb_for_subject(&left.subject),
            left_predicate,
            right_predicate
        );
    }
    let is_state = |verb: &str| matches!(verb, "is" | "are");
    if (is_trait(&left.verb) && is_state(right_verb))
        || (is_state(&left.verb) && is_trait(right_verb))
    {
        return format!(
            "As long as {condition}, {} {} {} and {} {}",
            lowercase_first(&left.subject),
            left.verb,
            left_predicate,
            right_verb,
            right_predicate
        );
    }
    format!(
        "{} {} {} and {} {} as long as {}",
        left.subject, left.verb, left_predicate, right_verb, right_predicate, condition
    )
}

pub(super) fn during_your_turn_subject_and_verb(subject: &str, verb: &str) -> (String, String) {
    let mut subject = lowercase_first(subject);
    let mut verb = verb.to_string();

    if let Some(rest) = subject.strip_prefix("each creature ") {
        subject = format!("creatures {rest}");
        if verb == "gets" {
            verb = "get".to_string();
        } else if verb == "has" {
            verb = "have".to_string();
        }
    }

    (subject, verb)
}

pub(super) fn normalize_keyword_predicate_case(predicate: &str) -> String {
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
    if let Some(keyword) = trimmed.strip_suffix(" as long as it's your turn")
        && is_keyword_phrase(keyword)
    {
        return format!("{} as long as it's your turn", keyword.to_ascii_lowercase());
    }
    if let Some((keyword, condition)) = trimmed.split_once(" as long as ")
        && is_keyword_phrase(keyword)
    {
        return format!("{} as long as {condition}", keyword.to_ascii_lowercase());
    }
    if let Some(keywords) = trimmed.strip_suffix(" until end of turn")
        && let Some(joined) = normalize_keyword_list_phrase(keywords)
    {
        return format!("{joined} until end of turn");
    }
    trimmed.to_string()
}

pub(super) fn normalize_keyword_list_phrase(text: &str) -> Option<String> {
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

pub(super) fn normalize_keyword_and_phrase(text: &str) -> Option<String> {
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

pub(super) fn merge_adjacent_subject_predicate_lines(lines: Vec<String>) -> Vec<String> {
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
                let counter_clause =
                    right
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
        // "Enchanted creature gets +1/+0." + "Enchanted creature can't be
        // blocked." → "Enchanted creature gets +1/+0 and can't be blocked."
        if idx + 1 < lines.len()
            && let Some((left_subject, left_verb, left_rest)) =
                split_subject_predicate_clause(&lines[idx])
            && matches!(
                left_verb,
                "gets" | "get" | "has" | "have" | "gains" | "gain"
            )
        {
            let right = lines[idx + 1].trim().trim_end_matches('.');
            let cant_prefix = format!("{left_subject} can't ");
            let plain = |text: &str| {
                let lower = text.to_ascii_lowercase();
                !lower.contains(", if ")
                    && !lower.contains(". ")
                    && !lower.contains("otherwise")
                    && !lower.contains("as long as")
                    && !lower.contains(" until ")
                    && !lower.contains(" unless ")
            };
            if let Some(cant_rest) = right.strip_prefix(&cant_prefix)
                && plain(left_rest)
                && plain(cant_rest)
            {
                let left_line = lines[idx].trim().trim_end_matches('.');
                merged.push(format!("{left_line} and can't {cant_rest}"));
                idx += 2;
                continue;
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
            && conditioned_subjects_equivalent(left_subject, right_subject)
            && can_merge_subject_predicates(left_verb, right_verb)
        {
            if lines[idx].contains(", if ") && lines[idx + 1].contains(", if ") {
                merged.push(lines[idx].clone());
                idx += 1;
                continue;
            }
            if let (Some(left_conditional), Some(right_conditional)) = (
                parse_conditional_subject_predicate(&lines[idx]),
                parse_conditional_subject_predicate(&lines[idx + 1]),
            ) {
                if conditioned_subjects_equivalent(
                    &left_conditional.subject,
                    &right_conditional.subject,
                ) && left_conditional
                    .condition
                    .eq_ignore_ascii_case(&right_conditional.condition)
                    && can_merge_subject_predicates(&left_conditional.verb, &right_conditional.verb)
                {
                    let left_predicate =
                        normalize_keyword_predicate_case(&left_conditional.predicate);
                    let right_predicate =
                        normalize_keyword_predicate_case(&right_conditional.predicate);
                    let right_verb = if matches!(
                        right_conditional.verb.as_str(),
                        "has" | "have" | "gains" | "gain"
                    ) {
                        have_verb_for_subject(&left_conditional.subject).to_string()
                    } else {
                        right_conditional.verb.clone()
                    };
                    merged.push(format_conditioned_subject_predicate_merge(
                        &left_conditional,
                        &left_predicate,
                        &right_verb,
                        &right_predicate,
                    ));
                    idx += 2;
                    continue;
                }
                if can_merge_conditional_state_bundle(&left_conditional, &right_conditional) {
                    merged.push(lines[idx].clone());
                    idx += 1;
                    continue;
                }
            }
            if parse_conditional_subject_predicate(&lines[idx]).is_some()
                || parse_conditional_subject_predicate(&lines[idx + 1]).is_some()
            {
                merged.push(lines[idx].clone());
                idx += 1;
                continue;
            }
            let has_sentence_branch = |text: &str| {
                let lower = text.to_ascii_lowercase();
                lower.contains("otherwise")
            };
            if has_sentence_branch(&lines[idx]) || has_sentence_branch(&lines[idx + 1]) {
                merged.push(lines[idx].clone());
                idx += 1;
                continue;
            }
            let left_raw = left_rest.trim_end_matches('.').trim();
            let right_raw = right_rest.trim_end_matches('.').trim();
            let has_conditional_tail = |text: &str| {
                let lower = text.to_ascii_lowercase();
                lower.contains(" as long as ") || lower.contains(" for as long as ")
            };
            if has_conditional_tail(left_raw) || has_conditional_tail(right_raw) {
                merged.push(lines[idx].clone());
                idx += 1;
                continue;
            }
            let is_trait = |verb: &str| matches!(verb, "has" | "have" | "gains" | "gain");
            if is_trait(left_verb) && is_trait(right_verb) {
                let left_lower = left_raw.to_ascii_lowercase();
                let right_lower = right_raw.to_ascii_lowercase();
                if left_lower.contains(" as long as ")
                    || right_lower.contains(" as long as ")
                    || left_lower.contains(" for as long as ")
                    || right_lower.contains(" for as long as ")
                {
                    merged.push(lines[idx].clone());
                    idx += 1;
                    continue;
                }
            }
            let left_rest =
                quote_granted_triggered_ability(&normalize_keyword_predicate_case(left_raw));
            let right_rest =
                quote_granted_triggered_ability(&normalize_keyword_predicate_case(right_raw));
            if is_trait(left_verb)
                && is_trait(right_verb)
                && left_verb.eq_ignore_ascii_case(right_verb)
                && let (Some(left_quote), Some(right_quote)) = (
                    trim_quoted_ability_sentence_end(&left_rest),
                    quoted_ability_text(&right_rest),
                )
            {
                merged.push(format!(
                    "{left_subject} {left_verb} {left_quote} and {right_quote}"
                ));
                idx += 2;
                continue;
            }
            if is_trait(left_verb)
                && is_trait(right_verb)
                && left_verb.eq_ignore_ascii_case(right_verb)
            {
                merged.push(format!(
                    "{left_subject} {left_verb} {left_rest} and {right_rest}"
                ));
            } else {
                merged.push(format!(
                    "{left_subject} {left_verb} {left_rest} and {right_verb} {right_rest}"
                ));
            }
            idx += 2;
            continue;
        }
        merged.push(lines[idx].clone());
        idx += 1;
    }

    merged
}

fn trim_quoted_ability_sentence_end(text: &str) -> Option<String> {
    let inner = quoted_ability_inner_text(text)?;
    Some(format!("\"{}\"", inner.trim_end_matches('.')))
}

fn quoted_ability_text(text: &str) -> Option<String> {
    quoted_ability_inner_text(text).map(|inner| format!("\"{inner}\""))
}

fn quote_granted_triggered_ability(text: &str) -> String {
    let trimmed = text.trim();
    if quoted_ability_text(trimmed).is_some() {
        return trimmed.to_string();
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("whenever ")
        || lower.starts_with("when ")
        || lower.starts_with("at the beginning ")
    {
        return format!("\"{}\"", capitalize_first(trimmed));
    }
    trimmed.to_string()
}

fn quoted_ability_inner_text(text: &str) -> Option<&str> {
    let inner = text.trim().strip_prefix('"')?.strip_suffix('"')?.trim();
    if inner.is_empty() || !inner.contains(':') {
        return None;
    }
    Some(inner)
}

pub(super) fn merge_blockability_lines(lines: Vec<String>) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    let block_this_turn_tail = " creatures can't block this turn";
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
            let left_no_period = left.trim_end_matches('.');
            let right_no_period = right.trim_end_matches('.');
            if let (Some(left_subject), Some(right_subject)) = (
                left_no_period.strip_suffix(block_this_turn_tail),
                right_no_period.strip_suffix(block_this_turn_tail),
            ) {
                if !left_subject.is_empty() && !right_subject.is_empty() {
                    merged.push(format!(
                        "{left_subject} creatures and {right_subject} creatures can't block this turn"
                    ));
                    idx += 2;
                    continue;
                }
            }
        }
        merged.push(lines[idx].clone());
        idx += 1;
    }
    merged
}

pub(super) fn merge_attached_transform_keyword_loss_lines(lines: Vec<String>) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;

    while idx < lines.len() {
        let mut consumed = 0usize;
        let mut subject: Option<String> = None;
        let mut base_pt: Option<String> = None;
        let mut replacement_subtypes: Option<String> = None;
        let mut granted_keywords: Vec<String> = Vec::new();
        let mut loses_all_abilities = false;

        while idx + consumed < lines.len() && consumed < 5 {
            let line = lines[idx + consumed].trim().trim_end_matches('.');
            let candidate_subject =
                if let Some(loss_subject) = split_lose_all_abilities_clause(line) {
                    loses_all_abilities = true;
                    Some(loss_subject)
                } else if let Some((have_subject, keyword)) = split_have_clause(line) {
                    granted_keywords.push(normalize_keyword_predicate_case(&keyword));
                    Some(have_subject)
                } else if let Some((predicate_subject, verb, predicate)) =
                    split_subject_predicate_clause(line)
                {
                    match verb {
                        "has" | "have" => {
                            if let Some(pt) = predicate.strip_prefix("base power and toughness ") {
                                base_pt = Some(pt.trim().to_string());
                                Some(predicate_subject.to_string())
                            } else {
                                break;
                            }
                        }
                        "is" | "are" => {
                            let lower = predicate.to_ascii_lowercase();
                            if lower.contains("in addition to")
                                || matches!(
                                    lower.as_str(),
                                    "creature"
                                        | "artifact"
                                        | "enchantment"
                                        | "land"
                                        | "planeswalker"
                                        | "battle"
                                        | "colorless"
                                        | "white"
                                        | "blue"
                                        | "black"
                                        | "red"
                                        | "green"
                                )
                            {
                                break;
                            }
                            replacement_subtypes = Some(lower);
                            Some(predicate_subject.to_string())
                        }
                        _ => break,
                    }
                } else {
                    break;
                };

            let Some(candidate_subject) = candidate_subject else {
                break;
            };
            if let Some(subject) = &subject {
                if !conditioned_subjects_equivalent(subject, &candidate_subject) {
                    break;
                }
            } else {
                subject = Some(candidate_subject);
            }
            consumed += 1;
        }

        if consumed >= 4
            && loses_all_abilities
            && base_pt.is_some()
            && replacement_subtypes.is_some()
            && !granted_keywords.is_empty()
            && subject
                .as_deref()
                .is_some_and(|subject| !subject_is_plural(subject))
        {
            let subject = subject.expect("merged transform group has a subject");
            let replacement_subtypes = replacement_subtypes.expect("checked replacement subtype");
            let base_pt = base_pt.expect("checked base pt");
            let subtype_phrase = capitalize_first(&replacement_subtypes);
            let article = indefinite_article_for_phrase(&replacement_subtypes);

            merged.push(format!(
                "{subject} is {article} {subtype_phrase} with base power and toughness {base_pt}."
            ));
            merged.push(format!(
                "It has {} and loses all other abilities.",
                join_with_and(&granted_keywords)
            ));
            idx += consumed;
            continue;
        }

        merged.push(lines[idx].clone());
        idx += 1;
    }

    merged
}

pub(super) fn merge_lose_all_transform_lines(lines: Vec<String>) -> Vec<String> {
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
                named = Some(title_case_transform_phrase(name.trim()));
                consumed += 1;
                continue;
            }

            collect_transform_descriptor_parts(
                rest,
                &mut colors,
                &mut card_types,
                &mut subtypes,
                &mut named,
                &mut base_pt,
            );
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
            descriptor.push_str(
                &subtypes
                    .iter()
                    .map(|subtype| title_case_transform_phrase(subtype))
                    .collect::<Vec<_>>()
                    .join(" "),
            );
        }
        if !card_types.is_empty() {
            if !descriptor.is_empty() {
                descriptor.push(' ');
            }
            descriptor.push_str(&card_types.join(" "));
        }
        if !descriptor.is_empty() {
            combined.push_str(" and is ");
            combined.push_str(indefinite_article_for_phrase(&descriptor));
            combined.push(' ');
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
        combined.push('.');

        merged.push(combined);
        idx += consumed;
    }

    merged
}

fn collect_transform_descriptor_parts(
    text: &str,
    colors: &mut Vec<String>,
    card_types: &mut Vec<String>,
    subtypes: &mut Vec<String>,
    named: &mut Option<String>,
    base_pt: &mut Option<String>,
) {
    let (text, parsed_base) = split_inline_transform_base_pt(text);
    if let Some((pt, card_type)) = parsed_base {
        *base_pt = Some(pt);
        if let Some(card_type) = card_type {
            push_unique_lower(card_types, &card_type);
        }
    }

    let (text, parsed_name) = split_inline_transform_name(text);
    if let Some(name) = parsed_name {
        *named = Some(name);
    }

    for raw_part in text
        .split(" and ")
        .flat_map(|part| part.split(','))
        .map(trim_transform_connector)
        .filter(|part| !part.is_empty())
    {
        let mut part = raw_part;
        while let Some(rest) = part.strip_prefix("is ") {
            part = rest.trim();
        }

        let words = part.split_whitespace().collect::<Vec<_>>();
        let mut idx = 0usize;
        while idx < words.len() {
            let lower = words[idx].to_ascii_lowercase();
            if matches!(
                lower.as_str(),
                "white" | "blue" | "black" | "red" | "green" | "colorless"
            ) {
                push_unique_lower(colors, &lower);
                idx += 1;
            } else {
                break;
            }
        }

        let mut remainder = words[idx..].join(" ");
        while let Some(rest) = remainder.strip_prefix("is ") {
            remainder = rest.trim().to_string();
        }
        let lower = trim_transform_connector(&remainder).to_ascii_lowercase();
        if lower.is_empty() {
            continue;
        }
        if let Some((subtype, card_type)) = split_transform_subtype_card_type(&lower) {
            if !subtype.is_empty() {
                push_unique_lower(subtypes, &subtype);
            }
            push_unique_lower(card_types, &card_type);
            continue;
        }
        if matches!(
            lower.as_str(),
            "creature" | "artifact" | "enchantment" | "land" | "planeswalker" | "battle"
        ) {
            push_unique_lower(card_types, &lower);
        } else {
            push_unique_lower(subtypes, &lower);
        }
    }
}

fn split_inline_transform_base_pt(text: &str) -> (&str, Option<(String, Option<String>)>) {
    let lower = text.to_ascii_lowercase();
    for marker in [
        " has base power and toughness ",
        " has base power, and toughness ",
        " has base power toughness ",
    ] {
        if let Some(idx) = lower.find(marker) {
            let tail = text[idx + marker.len()..].trim();
            let mut words = tail.split_whitespace();
            let Some(pt) = words.next() else {
                return (&text[..idx], None);
            };
            if !pt.contains('/') {
                return (&text[..idx], None);
            }
            let card_type = words.next().and_then(|word| {
                let lower = word.trim_end_matches('.').to_ascii_lowercase();
                matches!(
                    lower.as_str(),
                    "creature" | "artifact" | "enchantment" | "land" | "planeswalker" | "battle"
                )
                .then_some(lower)
            });
            return (
                &text[..idx],
                Some((pt.trim_end_matches('.').to_string(), card_type)),
            );
        }
    }
    (text, None)
}

fn split_inline_transform_name(text: &str) -> (&str, Option<String>) {
    let lower = text.to_ascii_lowercase();
    if let Some(name) = lower
        .strip_prefix("named ")
        .map(|_| trim_transform_connector(&text["named ".len()..]))
    {
        return ("", Some(title_case_transform_phrase(name)));
    }
    for marker in [" is named ", " named "] {
        if let Some(idx) = lower.find(marker) {
            let name = trim_transform_connector(&text[idx + marker.len()..]);
            return (&text[..idx], Some(title_case_transform_phrase(name)));
        }
    }
    (text, None)
}

fn trim_transform_connector(text: &str) -> &str {
    let mut trimmed = text.trim();
    loop {
        if let Some(rest) = trimmed.strip_prefix("and ") {
            trimmed = rest.trim();
            continue;
        }
        if let Some(rest) = trimmed.strip_suffix(" and") {
            trimmed = rest.trim();
            continue;
        }
        break trimmed;
    }
}

fn split_transform_subtype_card_type(text: &str) -> Option<(String, String)> {
    for glue in [" is ", " "] {
        let Some((left, right)) = text.rsplit_once(glue) else {
            continue;
        };
        if matches!(
            right,
            "creature" | "artifact" | "enchantment" | "land" | "planeswalker" | "battle"
        ) && !left.trim().is_empty()
        {
            return Some((left.trim().to_string(), right.to_string()));
        }
    }
    None
}

fn push_unique_lower(items: &mut Vec<String>, value: &str) {
    if !items.iter().any(|item| item.eq_ignore_ascii_case(value)) {
        items.push(value.to_string());
    }
}

fn title_case_transform_phrase(text: &str) -> String {
    text.split_whitespace()
        .map(capitalize_first)
        .collect::<Vec<_>>()
        .join(" ")
}

fn collect_transform_keyword_or_subtype(
    predicate: &str,
    granted_keywords: &mut Vec<String>,
    replacement_subtype: &mut Option<String>,
) -> bool {
    let mut matched = false;
    for part in predicate
        .split(" and ")
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        if is_keyword_phrase(part) {
            let keyword = normalize_keyword_predicate_case(part);
            if !granted_keywords
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(&keyword))
            {
                granted_keywords.push(keyword);
            }
            matched = true;
            continue;
        }

        let subtype = part.strip_prefix("is ").unwrap_or(part).trim();
        let subtype = strip_leading_article(subtype).trim();
        let lower = subtype.to_ascii_lowercase();
        if lower.is_empty()
            || matches!(
                lower.as_str(),
                "creature"
                    | "artifact"
                    | "enchantment"
                    | "land"
                    | "planeswalker"
                    | "battle"
                    | "white"
                    | "blue"
                    | "black"
                    | "red"
                    | "green"
                    | "colorless"
            )
        {
            return false;
        }
        if replacement_subtype
            .as_ref()
            .is_some_and(|existing| !existing.eq_ignore_ascii_case(&lower))
        {
            return false;
        }
        *replacement_subtype = Some(lower);
        matched = true;
    }
    matched
}

pub(super) fn merge_base_pt_loss_transform_lines(lines: Vec<String>) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;

    while idx < lines.len() {
        let first = lines[idx].trim().trim_end_matches('.');
        let Some((subject, verb, predicate)) = split_subject_predicate_clause(first) else {
            merged.push(lines[idx].clone());
            idx += 1;
            continue;
        };
        if !matches!(verb, "has" | "have") || !predicate.starts_with("base power and toughness ") {
            merged.push(lines[idx].clone());
            idx += 1;
            continue;
        }
        if subject_is_plural(subject) {
            merged.push(lines[idx].clone());
            idx += 1;
            continue;
        }

        let base_pt = predicate["base power and toughness ".len()..]
            .trim()
            .to_string();
        if idx + 1 >= lines.len() {
            merged.push(lines[idx].clone());
            idx += 1;
            continue;
        }
        let second = lines[idx + 1].trim().trim_end_matches('.');
        let Some(loss_subject) = split_lose_all_abilities_clause(second) else {
            merged.push(lines[idx].clone());
            idx += 1;
            continue;
        };
        if !conditioned_subjects_equivalent(subject, &loss_subject) {
            merged.push(lines[idx].clone());
            idx += 1;
            continue;
        }

        let mut consumed = 2usize;
        let mut replacement_subtype: Option<String> = None;
        let mut granted_keywords: Vec<String> = Vec::new();
        while idx + consumed < lines.len() && consumed < 5 {
            let line = lines[idx + consumed].trim().trim_end_matches('.');
            let Some((next_subject, next_verb, next_predicate)) =
                split_subject_predicate_clause(line)
            else {
                break;
            };
            if !conditioned_subjects_equivalent(subject, next_subject) {
                break;
            }
            if !matches!(next_verb, "has" | "have" | "gains" | "gain" | "is" | "are")
                || !collect_transform_keyword_or_subtype(
                    next_predicate,
                    &mut granted_keywords,
                    &mut replacement_subtype,
                )
            {
                break;
            }
            consumed += 1;
        }

        let Some(replacement_subtype) = replacement_subtype else {
            merged.push(lines[idx].clone());
            idx += 1;
            continue;
        };
        if granted_keywords.is_empty() {
            merged.push(lines[idx].clone());
            idx += 1;
            continue;
        }

        merged.push(format!(
            "{subject} is {} {} with base power and toughness {base_pt}.",
            indefinite_article_for_phrase(&replacement_subtype),
            capitalize_first(&replacement_subtype)
        ));
        merged.push(format!(
            "It has {} and loses all other abilities.",
            join_with_and(&granted_keywords)
        ));
        idx += consumed;
    }

    merged
}

pub(super) fn parse_simple_mana_add_line(line: &str) -> Option<(&str, &str)> {
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

pub(super) fn format_mana_symbol_alternatives(symbols: &[String]) -> String {
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

pub(super) fn merge_adjacent_simple_mana_add_lines(lines: Vec<String>) -> Vec<String> {
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

pub(super) fn have_verb_for_subject(subject: &str) -> &'static str {
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

fn merge_line_conditions_compatible(left: &str, right: &str) -> bool {
    match (
        parse_conditional_subject_predicate(left),
        parse_conditional_subject_predicate(right),
    ) {
        (Some(left_conditional), Some(right_conditional)) => left_conditional
            .condition
            .eq_ignore_ascii_case(&right_conditional.condition),
        (Some(_), None) | (None, Some(_)) => false,
        (None, None) => true,
    }
}

fn parse_during_your_turn_keyword_grant(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim().trim_end_matches('.');
    let body = if let Some(body) = trimmed.strip_prefix("During your turn, ") {
        body
    } else {
        trimmed
    };
    let (subject, verb, predicate) = split_subject_predicate_clause(body)?;
    if !matches!(verb, "has" | "have" | "gains" | "gain") {
        return None;
    }
    let predicate = if body == trimmed {
        predicate.strip_suffix(" as long as it's your turn")?
    } else {
        predicate
    };
    let keyword = normalize_keyword_predicate_case(predicate.trim());
    if !is_keyword_phrase(&keyword) {
        return None;
    }
    Some((subject.trim().to_string(), keyword))
}

fn parse_other_turns_keyword_grant(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim().trim_end_matches('.');
    let (subject, verb, predicate) = split_subject_predicate_clause(trimmed)?;
    if !matches!(verb, "has" | "have" | "gains" | "gain") {
        return None;
    }
    let predicate = predicate
        .trim()
        .strip_suffix(" during turns other than yours")?;
    let keyword = normalize_keyword_predicate_case(predicate.trim());
    if !is_keyword_phrase(&keyword) {
        return None;
    }
    Some((subject.trim().to_string(), keyword))
}

fn merge_complementary_turn_keyword_grants(left: &str, right: &str) -> Option<String> {
    let (left_subject, left_keyword) = parse_during_your_turn_keyword_grant(left)?;
    let (right_subject, right_keyword) = parse_other_turns_keyword_grant(right)?;
    if !conditioned_subjects_equivalent(&left_subject, &right_subject) {
        return None;
    }
    let subject = capitalize_first(&left_subject);
    let verb = have_verb_for_subject(&left_subject);
    let otherwise_subject = if subject_is_plural(&left_subject) {
        "they"
    } else {
        "it"
    };
    let otherwise_verb = if subject_is_plural(&left_subject) {
        "have"
    } else {
        "has"
    };
    Some(format!(
        "{subject} {verb} {left_keyword} during your turn. Otherwise, {otherwise_subject} {otherwise_verb} {right_keyword}."
    ))
}

pub(super) fn merge_subject_has_keyword_lines(lines: Vec<String>) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    while idx < lines.len() {
        if idx + 1 < lines.len() {
            let left = lines[idx].trim();
            let right = lines[idx + 1].trim();
            if let Some(compact) = merge_complementary_turn_keyword_grants(left, right) {
                merged.push(compact);
                idx += 2;
                continue;
            }
            if let (Some(left_conditional), Some(right_conditional)) = (
                parse_conditional_subject_predicate(left),
                parse_conditional_subject_predicate(right),
            ) && conditioned_subjects_equivalent(
                &left_conditional.subject,
                &right_conditional.subject,
            ) && left_conditional
                .condition
                .eq_ignore_ascii_case(&right_conditional.condition)
                && can_merge_subject_predicates(&left_conditional.verb, &right_conditional.verb)
            {
                let left_predicate = normalize_keyword_predicate_case(&left_conditional.predicate);
                let right_predicate =
                    normalize_keyword_predicate_case(&right_conditional.predicate);
                let right_verb = if matches!(
                    right_conditional.verb.as_str(),
                    "has" | "have" | "gains" | "gain"
                ) {
                    have_verb_for_subject(&left_conditional.subject).to_string()
                } else {
                    right_conditional.verb.clone()
                };
                merged.push(format_conditioned_subject_predicate_merge(
                    &left_conditional,
                    &left_predicate,
                    &right_verb,
                    &right_predicate,
                ));
                idx += 2;
                continue;
            }
            if let Some((left_condition, left_body)) = left.split_once(", ")
                && let Some((right_condition, right_body)) = right.split_once(", ")
                && left_condition.eq_ignore_ascii_case(right_condition)
                && left_condition
                    .to_ascii_lowercase()
                    .starts_with("as long as ")
                && let Some((left_subject, left_tail)) = split_have_clause(left_body)
                && let Some(right_subject) = right_body
                    .strip_suffix(" can't be blocked")
                    .or_else(|| right_body.strip_suffix(" cant be blocked"))
                && left_subject.eq_ignore_ascii_case(right_subject.trim())
            {
                let verb = have_verb_for_subject(&left_subject);
                let left_tail = normalize_keyword_predicate_case(&left_tail);
                merged.push(format!(
                    "{left_condition}, {left_subject} {verb} {left_tail} and can't be blocked"
                ));
                idx += 2;
                continue;
            }
            if parse_conditional_subject_predicate(left).is_some()
                || parse_conditional_subject_predicate(right).is_some()
            {
                merged.push(lines[idx].clone());
                idx += 1;
                continue;
            }
            if let Some((left_subject, left_tail)) = split_have_clause(left)
                && let Some((right_subject, right_tail)) = split_have_clause(right)
                && left_subject.eq_ignore_ascii_case(&right_subject)
            {
                let verb = have_verb_for_subject(&left_subject);
                let left_tail =
                    normalize_have_tail_for_merge(&normalize_keyword_predicate_case(&left_tail));
                let right_tail =
                    normalize_have_tail_for_merge(&normalize_keyword_predicate_case(&right_tail));
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
                && merge_line_conditions_compatible(left, right)
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

fn normalize_have_tail_for_merge(tail: &str) -> String {
    let trimmed = tail.trim();
    if trimmed.starts_with('"')
        && trimmed.ends_with(".\"")
        && let Some(stripped) = trimmed.strip_suffix(".\"")
    {
        return format!("{stripped}\"");
    }
    trimmed.to_string()
}

pub(super) fn merge_subject_animation_lines(lines: Vec<String>) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;

    while idx < lines.len() {
        if let Some(line) = compact_single_line_plural_animation_bundle(&lines[idx]) {
            merged.push(line);
            idx += 1;
            continue;
        }

        if idx + 1 < lines.len()
            && let Some(line) =
                merge_animation_with_granted_trigger_line(&lines[idx], &lines[idx + 1])
        {
            merged.push(line);
            idx += 2;
            continue;
        }
        if let Some(start) = parse_conditional_subject_predicate(&lines[idx])
            && matches!(start.verb.as_str(), "is" | "are")
            && is_creature_addition_predicate(&start.predicate)
        {
            let mut consumed = 1usize;
            let mut replacement_subtypes: Option<String> = None;
            let mut base_pt: Option<String> = None;
            let mut granted_predicates: Vec<String> = Vec::new();

            while idx + consumed < lines.len() {
                let Some(next) = parse_conditional_subject_predicate(&lines[idx + consumed]) else {
                    break;
                };
                if !start.subject.eq_ignore_ascii_case(&next.subject)
                    || !start.condition.eq_ignore_ascii_case(&next.condition)
                {
                    break;
                }

                match next.verb.as_str() {
                    "is" | "are" => {
                        if is_creature_addition_predicate(&next.predicate) {
                            consumed += 1;
                            continue;
                        }
                        if let Some(subtypes) = subtype_addition_predicate(&next.predicate) {
                            if replacement_subtypes.is_none() {
                                replacement_subtypes = Some(subtypes);
                                consumed += 1;
                                continue;
                            }
                            break;
                        }
                        if next
                            .predicate
                            .to_ascii_lowercase()
                            .contains("in addition to")
                        {
                            break;
                        }
                        if replacement_subtypes.is_none() {
                            replacement_subtypes = Some(next.predicate.clone());
                            consumed += 1;
                            continue;
                        }
                        break;
                    }
                    "has" | "have" | "gains" | "gain" => {
                        if let Some(pt) = next.predicate.strip_prefix("base power and toughness ") {
                            base_pt = Some(pt.trim().to_string());
                            consumed += 1;
                            continue;
                        }
                        granted_predicates.push(normalize_keyword_predicate_case(&next.predicate));
                        consumed += 1;
                        continue;
                    }
                    _ => break,
                }
            }

            if let Some(replacement_subtypes) = replacement_subtypes
                && (base_pt.is_some() || !granted_predicates.is_empty())
            {
                let plural_subject = start.verb == "are"
                    || start.verb == "have"
                    || subject_is_plural(&start.subject);
                let condition = if start
                    .condition
                    .eq_ignore_ascii_case("As long as it's your turn")
                {
                    "During your turn".to_string()
                } else {
                    start.condition.clone()
                };
                let each_subject = plural_subject.then(|| {
                    format!(
                        "each {}",
                        lowercase_first(&singularize_filter_subject(&start.subject))
                    )
                });
                let subject = each_subject
                    .as_deref()
                    .unwrap_or_else(|| start.subject.as_str());
                if !plural_subject {
                    let mut combined = format!(
                        "{condition}, {subject} is {} {replacement_subtypes}",
                        indefinite_article_for_phrase(&replacement_subtypes)
                    );
                    if let Some(pt) = base_pt {
                        combined.push_str(" with base power and toughness ");
                        combined.push_str(&pt);
                    }
                    if !granted_predicates.is_empty() {
                        combined.push_str(" and has ");
                        combined.push_str(&join_with_and(&granted_predicates));
                    }
                    merged.push(combined);
                    idx += consumed;
                    continue;
                }

                let mut descriptor = String::new();
                if let Some(pt) = base_pt {
                    descriptor.push_str(&pt);
                    descriptor.push(' ');
                }
                descriptor.push_str(&replacement_subtypes);

                let mut combined = format!("{condition}, {subject} is a {descriptor} creature");
                combined.push_str(" in addition to its other types");
                if !granted_predicates.is_empty() {
                    combined.push_str(" and has ");
                    combined.push_str(&join_with_and(&granted_predicates));
                }
                if start.subject.trim().eq_ignore_ascii_case("This creature") {
                    combined.push_str(". (It loses all other creature types.)");
                }
                merged.push(combined);
                idx += consumed;
                continue;
            }
        }

        if idx + 1 < lines.len()
            && let Some((left_subject, left_verb, left_rest)) =
                split_subject_predicate_clause(lines[idx].trim().trim_end_matches('.'))
            && matches!(left_verb, "is" | "are")
            && let Some(pt) = extract_base_pt_tail_for_subject(
                lines[idx + 1].trim().trim_end_matches('.'),
                left_subject,
            )
        {
            let lower_rest = left_rest.trim().to_ascii_lowercase();
            if lower_rest == "a creature in addition to its other types" {
                merged.push(format!(
                    "{left_subject} {left_verb} a {pt} creature in addition to its other types"
                ));
                idx += 2;
                continue;
            }
            if lower_rest == "creatures in addition to their other types" {
                if left_subject.trim().eq_ignore_ascii_case("Lands")
                    || left_subject.trim().eq_ignore_ascii_case("All lands")
                {
                    merged.push(format!(
                        "All lands {left_verb} {pt} creatures that are still lands"
                    ));
                    idx += 2;
                    continue;
                }
                merged.push(format!(
                    "{left_subject} {left_verb} {pt} creatures in addition to their other types"
                ));
                idx += 2;
                continue;
            }
        }

        // Single-line variant: "All lands are 1/1 creatures in addition to
        // their other types." → oracle's "... that are still lands."
        if let Some((subject, verb, rest)) =
            split_subject_predicate_clause(lines[idx].trim().trim_end_matches('.'))
            && matches!(verb, "is" | "are")
            && subject.trim().eq_ignore_ascii_case("All lands")
            && let Some(pt) = rest
                .trim()
                .strip_suffix(" creatures in addition to their other types")
        {
            merged.push(format!(
                "All lands {verb} {pt} creatures that are still lands"
            ));
            idx += 1;
            continue;
        }

        merged.push(lines[idx].clone());
        idx += 1;
    }

    merged
}

fn compact_single_line_plural_animation_bundle(line: &str) -> Option<String> {
    let trimmed = line.trim().trim_end_matches('.');
    let (condition, body) = trimmed.split_once(", ")?;
    let condition = condition.trim();
    if !condition.eq_ignore_ascii_case("During your turn")
        && !condition.to_ascii_lowercase().starts_with("as long as ")
    {
        return None;
    }

    let (subject, payload) = body.split_once(" are ")?;
    let payload = parse_plural_animation_payload(payload.trim())?;
    let subject = singularize_filter_subject(subject.trim());
    Some(format!(
        "{condition}, each {} is {payload}",
        lowercase_first(&subject)
    ))
}

fn merge_animation_with_granted_trigger_line(animation: &str, granted: &str) -> Option<String> {
    let animation = animation.trim().trim_end_matches('.');
    let granted = granted.trim().trim_end_matches('.');
    let (condition, animated_body) = animation.split_once(", ")?;
    if !condition.eq_ignore_ascii_case("During your turn") {
        return None;
    }
    let (animated_subject, animated_payload) = parse_animation_payload(animated_body)?;

    let (granted_body, granted_condition) = granted.rsplit_once(" As long as ")?;
    if !granted_condition.eq_ignore_ascii_case("it's your turn") {
        return None;
    }
    let (granted_subject, granted_ability) = granted_body.split_once(" have ")?;
    let granted_ability = granted_ability.trim();
    if !granted_ability.starts_with('"') || !granted_ability.ends_with('"') {
        return None;
    }
    if !animation_subjects_equivalent(&animated_subject, granted_subject) {
        return None;
    }

    let mut payload = normalize_animation_payload(animated_payload);
    let ability = granted_ability.trim_matches('"');
    payload.push_str(", and \"");
    payload.push_str(&capitalize_first(ability));
    payload.push('"');
    Some(format!("{condition}, {animated_subject} is {payload}"))
}

fn parse_animation_payload(animated_body: &str) -> Option<(String, String)> {
    if let Some((subject, payload)) = animated_body.split_once(" is ")
        && payload
            .to_ascii_lowercase()
            .contains("creature in addition to its other types")
    {
        return Some((subject.trim().to_string(), payload.trim().to_string()));
    }

    let (subject, payload) = animated_body.split_once(" are ")?;
    let canonical_subject = format!(
        "each {}",
        lowercase_first(&singularize_filter_subject(subject.trim()))
    );
    let canonical_payload = parse_plural_animation_payload(payload.trim())?;
    Some((canonical_subject, canonical_payload))
}

fn parse_plural_animation_payload(payload: &str) -> Option<String> {
    let rest = payload.strip_prefix("creatures in addition to their other types and have ")?;
    let rest = rest
        .strip_prefix("base power and toughness ")
        .or_else(|| rest.strip_prefix("base power and base toughness "))?;
    let (pt, rest) = rest.split_once(" and are ")?;
    let (subtypes, rest) = rest.split_once(" in addition to their other types")?;

    let mut canonical = format!(
        "a {} {} creature in addition to its other types",
        pt.trim(),
        singularize_terminal_subject_word(subtypes.trim())
    );
    let rest = rest.trim();
    if let Some(rest) = rest.strip_prefix("and ") {
        canonical.push_str(" and has ");
        canonical.push_str(&normalize_animation_grants(rest));
    }
    Some(canonical)
}

fn normalize_animation_payload(payload: String) -> String {
    payload.replace(" and have ", " and has ").replace(
        " and has indestructible and has haste",
        " and has indestructible, haste",
    )
}

fn normalize_animation_grants(grants: &str) -> String {
    let normalized = grants
        .trim()
        .replace("have ", "")
        .replace("has ", "")
        .replace(" and ", ", ");
    let normalized = capitalize_quoted_ability_starts(&normalized);
    if let Some(idx) = normalized.rfind(", \"") {
        let (head, tail) = normalized.split_at(idx);
        return format!("{head}, and {}", tail.trim_start_matches(", "));
    }
    normalized
}

fn capitalize_quoted_ability_starts(text: &str) -> String {
    let mut normalized = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        normalized.push(ch);
        if ch == '"'
            && let Some(next) = chars.next()
        {
            if next.is_ascii_lowercase() {
                normalized.push(next.to_ascii_uppercase());
            } else {
                normalized.push(next);
            }
        }
    }
    normalized
}

fn animation_subjects_equivalent(animated_subject: &str, granted_subject: &str) -> bool {
    let animated_lower = animated_subject.to_ascii_lowercase();
    let granted_lower = granted_subject.to_ascii_lowercase();
    if [animated_lower.as_str(), granted_lower.as_str()]
        .iter()
        .all(|subject| {
            subject.contains("non-equipment artifact")
                && subject.contains("non-aura enchantment")
                && subject.contains("mana value 4 or greater")
        })
    {
        return true;
    }

    fn normalize_subject(subject: &str) -> String {
        let normalized = subject
            .trim()
            .trim_start_matches("each ")
            .replace(" you control with mana value ", " with mana value ")
            .replace(
                " with mana value 4 or greater you control",
                " with mana value 4 or greater",
            )
            .replace("artifacts", "artifact")
            .replace("enchantments", "enchantment")
            .replace(" and ", " or ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase();
        let mut parts = normalized
            .split(" or ")
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        parts.sort_unstable();
        parts.join(" or ")
    }

    normalize_subject(animated_subject) == normalize_subject(granted_subject)
}

pub(super) fn drop_redundant_spell_cost_lines(lines: Vec<String>) -> Vec<String> {
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

pub(super) fn merge_conditioned_spell_and_activation_tax_lines(lines: Vec<String>) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    while idx < lines.len() {
        if idx + 1 < lines.len()
            && let Some(line) =
                merge_conditioned_spell_and_activation_tax_pair(&lines[idx], &lines[idx + 1])
        {
            merged.push(line);
            idx += 2;
            continue;
        }
        merged.push(lines[idx].clone());
        idx += 1;
    }
    merged
}

pub(super) fn merge_cast_permission_any_mana_lines(lines: Vec<String>) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;

    while idx < lines.len() {
        if idx + 1 < lines.len()
            && is_cast_permission_line(&lines[idx])
            && lines[idx + 1]
                .trim()
                .trim_end_matches('.')
                .eq_ignore_ascii_case("mana of any type can be spent to cast it")
        {
            let permission = lines[idx].trim().trim_end_matches('.');
            merged.push(format!(
                "{permission}, and mana of any type can be spent to cast it"
            ));
            idx += 2;
            continue;
        }

        merged.push(lines[idx].clone());
        idx += 1;
    }

    merged
}

fn is_cast_permission_line(line: &str) -> bool {
    let lower = line.trim().trim_end_matches('.').to_ascii_lowercase();
    lower.contains(" may cast ")
        && (lower.contains(" from among ") || lower.contains(" from "))
        && !lower.contains("mana of any type can be spent")
}

#[derive(Debug, Clone)]
struct SameTrueKeywordGrant {
    event: String,
    condition: String,
    condition_signature: String,
    subject: String,
    verb: String,
    keyword: String,
}

pub(super) fn merge_same_true_keyword_grant_lines(lines: Vec<String>) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;

    while idx < lines.len() {
        if let Some((line, consumed)) = merge_source_exiled_keyword_grant_lines(&lines, idx) {
            merged.push(line);
            idx += consumed;
            continue;
        }

        let Some(first) = parse_same_true_keyword_grant_line(&lines[idx]) else {
            merged.push(lines[idx].clone());
            idx += 1;
            continue;
        };

        let mut grants = vec![first];
        let mut consumed = 1usize;
        while idx + consumed < lines.len() {
            let Some(next) = parse_same_true_keyword_grant_line(&lines[idx + consumed]) else {
                break;
            };
            let start = &grants[0];
            if !start.event.eq_ignore_ascii_case(&next.event)
                || !start.subject.eq_ignore_ascii_case(&next.subject)
                || !start.verb.eq_ignore_ascii_case(&next.verb)
                || !start
                    .condition_signature
                    .eq_ignore_ascii_case(&next.condition_signature)
            {
                break;
            }
            grants.push(next);
            consumed += 1;
        }

        if grants.len() < 3 {
            merged.push(lines[idx].clone());
            idx += 1;
            continue;
        }

        let first = &grants[0];
        let first_condition = render_same_true_keyword_grant_condition(
            &first.condition,
            &first.keyword,
            &first.subject,
        );
        let remaining_keywords = grants[1..]
            .iter()
            .map(|grant| grant.keyword.clone())
            .collect::<Vec<_>>();
        merged.push(format!(
            "{}, {} {} {} until end of turn if {}. The same is true for {}.",
            first.event,
            first.subject,
            first.verb,
            first.keyword,
            first_condition,
            join_with_and(&remaining_keywords)
        ));
        idx += consumed;
    }

    merged
}

fn merge_source_exiled_keyword_grant_lines(
    lines: &[String],
    idx: usize,
) -> Option<(String, usize)> {
    let first = parse_source_exiled_keyword_grant_line(&lines[idx])?;
    let mut grants = vec![first];
    let mut consumed = 1usize;
    while idx + consumed < lines.len() {
        let Some(next) = parse_source_exiled_keyword_grant_line(&lines[idx + consumed]) else {
            break;
        };
        let start = &grants[0];
        if !start.subject.eq_ignore_ascii_case(&next.subject)
            || !start.verb.eq_ignore_ascii_case(&next.verb)
            || !start
                .condition_signature
                .eq_ignore_ascii_case(&next.condition_signature)
        {
            break;
        }
        grants.push(next);
        consumed += 1;
    }
    if grants.len() < 3 {
        return None;
    }

    let first = &grants[0];
    let source_surface = if lines[..idx]
        .iter()
        .rev()
        .take(3)
        .any(|line| line.trim().eq_ignore_ascii_case("Delve."))
    {
        "this creature's delve ability"
    } else {
        "this creature"
    };
    let remaining_keywords = grants[1..]
        .iter()
        .map(|grant| grant.keyword.clone())
        .collect::<Vec<_>>();
    let subject = lowercase_first(&first.subject);
    Some((
        format!(
            "If a creature card with {} was exiled with {}, {} {} {}. The same is true for {}.",
            first.keyword,
            source_surface,
            subject,
            first.verb,
            first.keyword,
            join_with_and(&remaining_keywords)
        ),
        consumed,
    ))
}

fn parse_same_true_keyword_grant_line(line: &str) -> Option<SameTrueKeywordGrant> {
    let trimmed = line.trim().trim_end_matches('.');
    let (event, rest) = trimmed.split_once(", if ")?;
    let (condition, effect) = rest.split_once(", ")?;
    let (subject, verb, predicate) = split_subject_predicate_clause(effect)?;
    if !matches!(verb, "gains" | "gain") {
        return None;
    }
    let keyword = predicate
        .trim()
        .strip_suffix(" until end of turn")?
        .trim()
        .to_ascii_lowercase();
    if !is_keyword_phrase(&keyword) || !condition_mentions_keyword(condition, &keyword) {
        return None;
    }
    let condition_signature = condition
        .to_ascii_lowercase()
        .replace(&keyword, "{keyword}");
    Some(SameTrueKeywordGrant {
        event: event.trim().to_string(),
        condition: condition.trim().to_string(),
        condition_signature,
        subject: subject.trim().to_string(),
        verb: verb.trim().to_string(),
        keyword,
    })
}

fn parse_source_exiled_keyword_grant_line(line: &str) -> Option<SameTrueKeywordGrant> {
    let trimmed = line.trim().trim_end_matches('.');
    let (effect, condition) = if let Some(rest) = trimmed
        .strip_prefix("As long as ")
        .or_else(|| trimmed.strip_prefix("as long as "))
    {
        let (condition, effect) = rest.split_once(", ")?;
        (effect, condition)
    } else {
        trimmed.split_once(" as long as ")?
    };
    let (subject, verb, predicate) = split_subject_predicate_clause(effect)?;
    if !matches!(verb, "has" | "have") {
        return None;
    }
    let keyword = predicate.trim().to_ascii_lowercase();
    if !is_keyword_phrase(&keyword) || !condition_mentions_keyword(condition, &keyword) {
        return None;
    }
    let condition_lower = condition.to_ascii_lowercase();
    if !condition_lower.contains("exiled with this creature") {
        return None;
    }
    let condition_signature = condition_lower.replace(&keyword, "{keyword}");
    Some(SameTrueKeywordGrant {
        event: String::new(),
        condition: condition.trim().to_string(),
        condition_signature,
        subject: subject.trim().to_string(),
        verb: verb.trim().to_string(),
        keyword,
    })
}

fn condition_mentions_keyword(condition: &str, keyword: &str) -> bool {
    let condition = condition.to_ascii_lowercase();
    condition.contains(&format!(" with {keyword}"))
        || condition.contains(&format!(" has {keyword}"))
}

fn render_same_true_keyword_grant_condition(
    condition: &str,
    keyword: &str,
    grant_subject: &str,
) -> String {
    let condition = condition.trim();
    if grant_subject.eq_ignore_ascii_case("creatures you control") {
        let control_pattern = format!("you control a creature with {keyword}");
        if condition.eq_ignore_ascii_case(&control_pattern) {
            return format!("a creature you control has {keyword}");
        }

        let graveyard_pattern =
            format!("you have a creature card with {keyword} in your graveyard");
        if condition.eq_ignore_ascii_case(&graveyard_pattern) {
            return format!("a creature card in your graveyard has {keyword}");
        }

        let graveyard_exists_pattern =
            format!("there is a creature card with {keyword} in your graveyard");
        if condition.eq_ignore_ascii_case(&graveyard_exists_pattern) {
            return format!("a creature card in your graveyard has {keyword}");
        }
    }
    condition.to_string()
}

fn merge_conditioned_spell_and_activation_tax_pair(first: &str, second: &str) -> Option<String> {
    let first = compact_merge_pass_whitespace(first)
        .trim_end_matches('.')
        .to_string();
    let second = compact_merge_pass_whitespace(second)
        .trim_end_matches('.')
        .to_string();
    let (first_prefix, first_body) = first.split_once(", ")?;
    let (second_prefix, second_body) = second.split_once(", ")?;
    if first_prefix != second_prefix {
        return None;
    }
    let first_lower = first_body.to_ascii_lowercase();
    let second_lower = second_body.to_ascii_lowercase();
    if !first_lower.contains("spells ")
        || !first_lower.contains(" cost ")
        || !first_lower.ends_with(" to cast")
        || !second_lower.starts_with("abilities ")
        || !second_lower.contains(" cost ")
        || !second_lower.contains(" to activate")
    {
        return None;
    }
    Some(format!("{first_prefix}, {first_body} and {second_body}"))
}

fn compact_merge_pass_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(super) fn is_keyword_style_line(line: &str) -> bool {
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
        "casualty ",
        "dash ",
        "echo ",
        "echo—",
        "echo-",
        "ward ",
        "ward--",
        "ward—",
        "kicker ",
        "bloodthirst ",
        "foretell ",
        "flashback ",
        "ninjutsu ",
        "cycling ",
        "landcycling ",
        "basic landcycling ",
        "madness ",
        "morph ",
        "suspend ",
        "prototype ",
        "bestow ",
        "affinity ",
        "ascend",
        "soulbond",
        "undaunted",
        "vanishing",
        "reinforce ",
        "scavenge ",
        "fuse",
        "adventure",
    ]
    .iter()
    .any(|prefix| lower.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_conditional_self_buff_and_granted_ability_keeps_hellbent_surface() {
        let merged = merge_subject_has_keyword_lines(vec![
            "This creature gets +1/+0 as long as you have no cards in hand.".to_string(),
            "As long as you have no cards in hand, this source has \"{B}: Regenerate this creature.\"".to_string(),
        ]);
        assert_eq!(
            merged,
            vec![
                "Hellbent \u{2014} As long as you have no cards in hand, this creature gets +1/+0 and has \"{B}: Regenerate this creature.\"".to_string()
            ]
        );
    }

    #[test]
    fn merge_subject_has_keyword_lines_compacts_complementary_turn_grants() {
        let merged = merge_subject_has_keyword_lines(vec![
            "During your turn, equipped creature has deathtouch.".to_string(),
            "Equipped creature has reach during turns other than yours.".to_string(),
        ]);
        assert_eq!(
            merged,
            vec![
                "Equipped creature has deathtouch during your turn. Otherwise, it has reach."
                    .to_string()
            ]
        );
    }

    #[test]
    fn merge_adjacent_subject_predicate_lines_does_not_merge_otherwise_branch() {
        let merged = merge_adjacent_subject_predicate_lines(vec![
            "Equipped creature gets +1/+1.".to_string(),
            "Equipped creature has deathtouch during your turn. Otherwise, it has reach."
                .to_string(),
        ]);
        assert_eq!(
            merged,
            vec![
                "Equipped creature gets +1/+1.".to_string(),
                "Equipped creature has deathtouch during your turn. Otherwise, it has reach."
                    .to_string(),
            ]
        );
    }

    #[test]
    fn merge_lose_all_transform_lines_classifies_colors_subtypes_names_and_base_pt() {
        let merged = merge_lose_all_transform_lines(vec![
            "Enchanted creature loses all abilities.".to_string(),
            "Enchanted creature is green is citizen.".to_string(),
            "Enchanted creature is white.".to_string(),
            "Enchanted creature is named legitimate businessperson.".to_string(),
            "Enchanted creature is creature.".to_string(),
            "Enchanted creature has base power and toughness 1/1.".to_string(),
        ]);
        assert_eq!(
            merged,
            vec![
                "Enchanted creature loses all abilities and is a green and white Citizen creature with base power and toughness 1/1 named Legitimate Businessperson."
                    .to_string()
            ]
        );
    }

    #[test]
    fn merge_lose_all_transform_lines_strips_repeated_is_before_subtype() {
        let merged = merge_lose_all_transform_lines(vec![
            "Enchanted creature loses all abilities.".to_string(),
            "Enchanted creature is is treefolk.".to_string(),
            "Enchanted creature is creature.".to_string(),
            "Enchanted creature has base power and toughness 0/4.".to_string(),
        ]);
        assert_eq!(
            merged,
            vec![
                "Enchanted creature loses all abilities and is a Treefolk creature with base power and toughness 0/4."
                    .to_string()
            ]
        );
    }

    #[test]
    fn merge_lose_all_transform_lines_splits_inline_name_and_base_pt() {
        let merged = merge_lose_all_transform_lines(vec![
            "Enchanted creature loses all abilities.".to_string(),
            "Enchanted creature is white and green citizen is creature named legitimate businessperson and has base power toughness 1/1.".to_string(),
        ]);
        assert_eq!(
            merged,
            vec![
                "Enchanted creature loses all abilities and is a white and green Citizen creature with base power and toughness 1/1 named Legitimate Businessperson."
                    .to_string()
            ]
        );
    }
}
