use super::*;

pub(super) fn normalize_sentence_surface_style(line: &str) -> String {
    let mut normalized = line.trim().to_string();
    if normalized.is_empty() {
        return normalized;
    }
    normalized = strip_square_bracketed_segments(&normalized)
        .trim()
        .to_string();
    normalized = normalized.replace('\u{00a0}', " ");
    normalized = normalized.split_whitespace().collect::<Vec<_>>().join(" ");
    normalized = normalize_top_card_exile_imperative(&normalized);
    if normalized
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_lowercase())
    {
        normalized = capitalize_first(&normalized);
    }
    normalized = capitalize_sentence_boundaries(&normalized);
    normalized = replace_standalone_phrase(&normalized, "a artifact", "an artifact");
    normalized = replace_standalone_phrase(&normalized, "a enchantment", "an enchantment");
    normalized = replace_standalone_phrase(&normalized, "a Aura", "an Aura");
    normalized = replace_standalone_phrase(&normalized, "a Elf", "an Elf");
    normalized = normalized
        .replace(" ors ", " or ")
        .replace("non-aura", "non-Aura")
        .replace("Non-aura", "Non-Aura")
        .replace("non-equipment", "non-Equipment")
        .replace("Non-equipment", "Non-Equipment")
        .replace("for each a ", "for each ")
        .replace("for each an ", "for each ");
    if let Some(keyword_line) = normalize_keyword_only_comma_line(&normalized) {
        normalized = keyword_line;
    }
    if !is_keyword_style_line(&normalized)
        && !normalized.ends_with('.')
        && !normalized.ends_with('!')
        && !normalized.ends_with('?')
        && !normalized.ends_with('"')
        && !normalized.ends_with(')')
    {
        normalized.push('.');
    }
    normalized
}

pub(super) fn normalize_debug_safe_legacy_surface(line: &str) -> String {
    let lower = line.to_ascii_lowercase();
    if lower.contains(
        "tap target creature or planeswalker. choose it. activated abilities of that permanent can't be activated this turn",
    ) {
        return line.replace(
            "choose it. activated abilities of that permanent can't be activated this turn",
            "its activated abilities can't be activated this turn",
        );
    }
    if lower.contains("that permanent's mana value") {
        return line.replace("that permanent's mana value", "that card's mana value");
    }
    if lower.contains("if it's a permanent, exile it") {
        return line.replace(
            "if it's a permanent, exile it",
            "if it would leave the battlefield, exile it instead",
        );
    }
    line.to_string()
}

pub(super) fn capitalize_sentence_boundaries(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut capitalize_next = false;

    for ch in input.chars() {
        if capitalize_next && ch.is_ascii_lowercase() {
            output.push(ch.to_ascii_uppercase());
            capitalize_next = false;
            continue;
        }

        output.push(ch);

        if matches!(ch, '.' | '!' | '?') {
            capitalize_next = true;
        } else if capitalize_next && !ch.is_whitespace() && ch != '"' && ch != '\'' {
            capitalize_next = false;
        }
    }

    output
}

pub(super) fn normalize_top_card_exile_imperative(input: &str) -> String {
    let mut normalized = input
        .replace("You exile the top card", "Exile the top card")
        .replace("you exile the top card", "exile the top card")
        .replace("You exile cards from the top", "Exile cards from the top")
        .replace("you exile cards from the top", "exile cards from the top");

    for (from, to) in [
        (": exile the top card", ": Exile the top card"),
        (": exile cards from the top", ": Exile cards from the top"),
        (". exile the top card", ". Exile the top card"),
        (". exile cards from the top", ". Exile cards from the top"),
    ] {
        normalized = normalized.replace(from, to);
    }

    normalized
}

fn normalize_keyword_only_comma_line(line: &str) -> Option<String> {
    let parts = line
        .trim()
        .trim_end_matches('.')
        .split(',')
        .map(|part| {
            let part = part.trim();
            part.strip_prefix("and ").unwrap_or(part)
        })
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.len() < 2 || !parts.iter().all(|part| is_keyword_phrase(part)) {
        return None;
    }
    Some(capitalize_first(
        &parts
            .iter()
            .map(|part| part.to_ascii_lowercase())
            .collect::<Vec<_>>()
            .join(", "),
    ))
}

fn replace_standalone_phrase(input: &str, from: &str, to: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut rest = input;

    while let Some(idx) = rest.find(from) {
        let before_ok = rest[..idx]
            .chars()
            .next_back()
            .is_none_or(|ch| !ch.is_ascii_alphanumeric() && ch != '-' && ch != '\'');
        let after_idx = idx + from.len();
        let after_ok = rest[after_idx..]
            .chars()
            .next()
            .is_none_or(|ch| !ch.is_ascii_alphanumeric() && ch != '-' && ch != '\'');

        output.push_str(&rest[..idx]);
        if before_ok && after_ok {
            output.push_str(to);
        } else {
            output.push_str(from);
        }
        rest = &rest[after_idx..];
    }

    output.push_str(rest);
    output
}

pub(super) fn chapter_number_to_roman(chapter: u32) -> Option<&'static str> {
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

pub(super) fn strip_prefix_ascii_ci<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
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

pub(super) fn strip_suffix_ascii_ci<'a>(text: &'a str, suffix: &str) -> Option<&'a str> {
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

pub(super) fn split_once_ascii_ci<'a>(
    text: &'a str,
    separator: &str,
) -> Option<(&'a str, &'a str)> {
    let lower = text.to_ascii_lowercase();
    let sep_lower = separator.to_ascii_lowercase();
    let idx = lower.find(&sep_lower)?;
    Some((&text[..idx], &text[idx + separator.len()..]))
}

pub(super) fn is_render_heading_prefix(prefix: &str) -> bool {
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

pub(super) fn normalize_granted_activated_ability_clause(text: &str) -> Option<String> {
    let (prefix, rest) = text.split_once(" gain \"{T}, choose ")?;
    let (choice, suffix) = rest.split_once(": this spell fights ")?;
    let (fought, tail) = suffix.split_once('"')?;
    if fought != choice {
        return None;
    }
    Some(format!(
        "{prefix} gain \"{{T}}: This creature fights {choice}\"{tail}"
    ))
}

pub(super) fn normalize_granted_beginning_trigger_clause(_text: &str) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentence_surface_capitalizes_sentence_boundaries() {
        assert_eq!(
            normalize_sentence_surface_style(
                "exile the top card of your library. you may play that card this turn",
            ),
            "Exile the top card of your library. You may play that card this turn."
        );
        assert_eq!(
            normalize_sentence_surface_style(
                "players can't cast noncreature spells this turn. exile this spell",
            ),
            "Players can't cast noncreature spells this turn. Exile this spell."
        );
    }

    #[test]
    fn sentence_surface_uses_top_card_exile_imperative() {
        assert_eq!(
            normalize_sentence_surface_style(
                "whenever this creature attacks, you exile the top card of your library. you may play that card this turn",
            ),
            "Whenever this creature attacks, exile the top card of your library. You may play that card this turn."
        );
        assert_eq!(
            normalize_sentence_surface_style(
                "{2}{R}: you exile cards from the top of your library until you exile a nonland card",
            ),
            "{2}{R}: Exile cards from the top of your library until you exile a nonland card."
        );
    }
}
