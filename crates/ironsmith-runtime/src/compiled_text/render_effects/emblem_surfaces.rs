use super::*;

fn is_phrase_boundary(text: &str, index: usize) -> bool {
    text[..index]
        .chars()
        .next_back()
        .is_none_or(|character| !character.is_alphanumeric())
}

fn replace_whole_phrase_case_insensitive(text: &str, needle: &str, replacement: &str) -> String {
    if needle.is_empty() {
        return text.to_string();
    }

    let lower_text = text.to_ascii_lowercase();
    let lower_needle = needle.to_ascii_lowercase();
    let mut rendered = String::with_capacity(text.len());
    let mut copied_until = 0;
    let mut search_from = 0;

    while let Some(relative_start) = lower_text[search_from..].find(&lower_needle) {
        let start = search_from + relative_start;
        let end = start + lower_needle.len();
        let starts_at_boundary = is_phrase_boundary(text, start);
        let ends_at_boundary = text[end..]
            .chars()
            .next()
            .is_none_or(|character| !character.is_alphanumeric());
        if starts_at_boundary && ends_at_boundary {
            rendered.push_str(&text[copied_until..start]);
            rendered.push_str(replacement);
            copied_until = end;
            search_from = end;
        } else {
            search_from = start + text[start..].chars().next().map_or(1, char::len_utf8);
        }
    }

    rendered.push_str(&text[copied_until..]);
    rendered
}

/// Prefer the parser-captured emblem rules sentences, while using typed trigger
/// filters to recover canonical subtype capitalization lost during lowering.
/// Separate quoted abilities are stored on separate lines, so preserve their
/// individual quote boundaries in the returned quote payload.
pub(super) fn stored_emblem_rules_text(
    emblem: &crate::effect::EmblemDescription,
) -> Option<String> {
    let mut text = emblem.text.trim().to_string();
    if text.is_empty() {
        return None;
    }

    let mut typed_subtypes = emblem
        .abilities
        .iter()
        .filter_map(|ability| {
            let AbilityKind::Triggered(triggered) = &ability.kind else {
                return None;
            };
            triggered
                .trigger
                .downcast_ref::<crate::triggers::SpellCastTrigger>()
                .and_then(|trigger| trigger.filter.as_ref())
        })
        .flat_map(|filter| filter.subtypes.iter().chain(&filter.excluded_subtypes))
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    typed_subtypes.sort_by_key(|subtype| std::cmp::Reverse(subtype.len()));
    typed_subtypes.dedup();

    for subtype in typed_subtypes {
        text = replace_whole_phrase_case_insensitive(&text, &subtype, &subtype);
    }

    let mut abilities = text
        .lines()
        .map(str::trim)
        .filter(|ability| !ability.is_empty())
        .map(|ability| {
            capitalize_first(
                ability.trim_end_matches(|character| matches!(character, '.' | '!' | '?')),
            )
        })
        .collect::<Vec<_>>();
    if abilities.len() == 1 {
        abilities[0] = ensure_trailing_period(&abilities[0]);
    } else if let Some(last) = abilities.last_mut() {
        *last = ensure_trailing_period(last);
    }
    (!abilities.is_empty()).then(|| abilities.join("\" and \""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_subtype_replacement_respects_word_boundaries() {
        assert_eq!(
            replace_whole_phrase_case_insensitive(
                "whenever you cast an elf spell, it grants itself haste",
                "Elf",
                "Elf",
            ),
            "whenever you cast an Elf spell, it grants itself haste"
        );
    }

    #[test]
    fn separate_emblem_abilities_keep_separate_quote_boundaries() {
        let emblem = crate::effect::EmblemDescription::new(
            "Test Emblem",
            "untap all permanents you control.\nyou draw a card.",
        );
        assert_eq!(
            stored_emblem_rules_text(&emblem).as_deref(),
            Some("Untap all permanents you control\" and \"You draw a card.")
        );
    }
}
