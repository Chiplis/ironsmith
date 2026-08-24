use super::*;

pub(in super::super) fn leading_explicit_name(words: &[&str]) -> Option<String> {
    let first = *words.first()?;
    if !simple_name_word(first)
        || explicit_name_descriptor(first)
        || is_token_pt(first)
        || is_card_type(first)
        || is_subtype(first)
    {
        return None;
    }

    let mut name_words = vec![first];
    for word in words.iter().skip(1) {
        if !simple_name_word(word)
            || explicit_name_descriptor(word)
            || is_token_pt(word)
            || is_card_type(word)
            || is_subtype(word)
        {
            break;
        }
        name_words.push(*word);
    }

    if name_words.len() >= 2
        || words
            .get(1)
            .is_some_and(|word| explicit_name_descriptor(word) || is_token_pt(word))
    {
        Some(title_case_words(&name_words))
    } else {
        None
    }
}

pub(in super::super) fn leading_name_phrase(words: &[&str]) -> Option<String> {
    let mut name_words = Vec::new();
    for word in words {
        if LEADING_NAME_STOP_WORDS.contains(word)
            || is_token_pt(word)
            || is_card_type(word)
            || !simple_name_word(word)
        {
            break;
        }
        name_words.push(*word);
    }

    (name_words.len() >= 2).then(|| title_case_words(&name_words))
}

pub(in super::super) fn vehicle_surface_name(words: &[&str], named: Option<&str>) -> String {
    if let Some(named) = named {
        return named.to_string();
    }
    for word in words {
        if is_token_pt(word)
            || !simple_name_word(word)
            || matches!(
                *word,
                "artifact"
                    | "token"
                    | "tokens"
                    | "vehicle"
                    | "colorless"
                    | "named"
                    | "with"
                    | "and"
                    | "crew"
                    | "flying"
                    | "white"
                    | "blue"
                    | "black"
                    | "red"
                    | "green"
            )
            || is_card_type(word)
            || is_subtype(word)
        {
            continue;
        }
        return title_case_words(&[*word]);
    }
    "Vehicle".to_string()
}

pub(in super::super) fn creature_surface_name(
    words: &[&str],
    named: Option<&str>,
    subtype_fallback: Option<&str>,
) -> String {
    named
        .map(str::to_string)
        .or_else(|| leading_name_phrase(words))
        .or_else(|| leading_explicit_name(words))
        .or_else(|| subtype_fallback.map(str::to_string))
        .unwrap_or_else(|| "OwnedLexToken".to_string())
}
