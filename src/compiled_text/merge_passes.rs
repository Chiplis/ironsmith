use super::*;

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
    if lower.starts_with("protection from ") {
        return true;
    }
    if lower.starts_with("ward ") {
        return true;
    }
    if lower == "sunburst" || lower.starts_with("fading ") || lower.starts_with("vanishing ") {
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
            | "partner"
            | "assist"
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
    for verb in [
        " gets ", " get ", " has ", " have ", " gains ", " gain ", " is ", " are ",
    ] {
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

    if let Some((condition, body)) = trimmed.split_once(", ")
        && condition.to_ascii_lowercase().starts_with("as long as ")
    {
        let (subject, verb, predicate) = split_subject_predicate_clause(body)?;
        return Some(ConditionalSubjectPredicate {
            condition: condition.trim().to_string(),
            subject: subject.trim().to_string(),
            verb: verb.trim().to_string(),
            predicate: predicate.trim().to_string(),
        });
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

fn can_merge_conditional_state_bundle(
    left: &ConditionalSubjectPredicate,
    right: &ConditionalSubjectPredicate,
) -> bool {
    matches!(left.verb.as_str(), "is" | "are")
        && matches!(right.verb.as_str(), "is" | "are")
        && left.subject.eq_ignore_ascii_case(&right.subject)
        && left.condition.eq_ignore_ascii_case(&right.condition)
}

pub(super) fn can_merge_subject_predicates(left_verb: &str, right_verb: &str) -> bool {
    let is_get = |verb: &str| matches!(verb, "gets" | "get");
    let is_trait = |verb: &str| matches!(verb, "has" | "have" | "gains" | "gain");
    let is_state = |verb: &str| matches!(verb, "is" | "are");

    (is_get(left_verb) && is_trait(right_verb))
        || (is_trait(left_verb) && is_get(right_verb))
        || (is_trait(left_verb) && is_trait(right_verb))
        || ((left_verb == "gets" && right_verb == "is")
            || (left_verb == "is" && right_verb == "gets"))
        || (is_state(left_verb) && is_state(right_verb))
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

pub(super) fn normalize_gains_tail(predicate: &str) -> String {
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

pub(super) fn merge_sentence_subject_predicates(line: &str) -> Option<String> {
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
            if let (Some(left_conditional), Some(right_conditional)) = (
                parse_conditional_subject_predicate(&lines[idx]),
                parse_conditional_subject_predicate(&lines[idx + 1]),
            ) && can_merge_conditional_state_bundle(&left_conditional, &right_conditional)
            {
                merged.push(lines[idx].clone());
                idx += 1;
                continue;
            }
            let left_raw = left_rest.trim_end_matches('.').trim();
            let right_raw = right_rest.trim_end_matches('.').trim();
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
            let left_rest = normalize_keyword_predicate_case(left_raw);
            let right_rest = normalize_keyword_predicate_case(right_raw);
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

pub(super) fn merge_blockability_lines(lines: Vec<String>) -> Vec<String> {
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

pub(super) fn merge_subject_has_keyword_lines(lines: Vec<String>) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    while idx < lines.len() {
        if idx + 1 < lines.len() {
            let left = lines[idx].trim();
            let right = lines[idx + 1].trim();
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

pub(super) fn normalize_repeated_has_keyword_list(tail: &str) -> String {
    let mut normalized = tail.trim().trim_end_matches('.').to_string();
    if normalized.is_empty() {
        return normalized;
    }
    normalized = normalized.replace(" and has ", " and ");
    normalized = normalized.replace(", has ", ", ");

    let mut parts: Vec<String> = if normalized.contains(',') {
        normalized
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(|part| part.trim_start_matches("and ").trim())
            .map(|part| part.to_string())
            .collect()
    } else {
        normalized
            .split(" and ")
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(|part| part.to_string())
            .collect()
    };

    if parts.len() < 2 {
        return normalized;
    }
    if !parts.iter().all(|part| is_keyword_phrase(part)) {
        return normalized;
    }
    for part in &mut parts {
        *part = part.to_ascii_lowercase();
    }

    if parts.len() == 2 {
        return format!("{} and {}", parts[0], parts[1]);
    }
    let last = parts.pop().unwrap_or_default();
    format!("{}, and {}", parts.join(", "), last)
}

pub(super) fn merge_subject_is_legendary_gets_then_has_lines(lines: Vec<String>) -> Vec<String> {
    if lines.len() != 2 {
        return lines;
    }
    let left = lines[0].trim().trim_end_matches('.');
    let right = lines[1].trim().trim_end_matches('.');

    let (right_subject, right_tail) = if let Some((subject, tail)) = right.split_once(" has ") {
        (subject.trim().to_string(), tail.trim().to_string())
    } else if let Some((subject, tail)) = right.split_once(" have ") {
        (subject.trim().to_string(), tail.trim().to_string())
    } else {
        return lines;
    };

    let (left_subject, left_rest) = if let Some((subject, rest)) = left.split_once(" is ") {
        (subject.trim(), rest.trim())
    } else {
        return lines;
    };
    if !left_subject.eq_ignore_ascii_case(&right_subject) {
        return lines;
    }

    let Some((state, gets_tail)) = left_rest.split_once(" and gets ") else {
        return lines;
    };
    if !state.trim().eq_ignore_ascii_case("legendary") {
        return lines;
    }
    let gets_tail = gets_tail.trim();
    if gets_tail.is_empty() {
        return lines;
    }

    let right_tail = normalize_repeated_has_keyword_list(&right_tail);
    let subject = right_subject;
    let verb = have_verb_for_subject(&subject);
    vec![format!(
        "{subject} is legendary, gets {gets_tail}, and {verb} {right_tail}."
    )]
}

pub(super) fn merge_subject_animation_lines(lines: Vec<String>) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;

    while idx < lines.len() {
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
                let mut payloads = Vec::new();
                if let Some(pt) = base_pt {
                    payloads.push(format!("base power and toughness {pt}"));
                }
                payloads.extend(granted_predicates);

                let mut combined = format!(
                    "{}, {} is {}",
                    start.condition,
                    start.subject,
                    with_indefinite_article(&replacement_subtypes)
                );
                if !payloads.is_empty() {
                    combined.push_str(" with ");
                    combined.push_str(&join_with_and(&payloads));
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
                merged.push(format!(
                    "{left_subject} {left_verb} {pt} creatures in addition to their other types"
                ));
                idx += 2;
                continue;
            }
        }

        merged.push(lines[idx].clone());
        idx += 1;
    }

    merged
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
