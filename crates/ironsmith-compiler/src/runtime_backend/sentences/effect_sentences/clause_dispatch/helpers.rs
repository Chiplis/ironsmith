use super::*;

pub(super) fn render_lower_words(tokens: &[OwnedLexToken]) -> String {
    let word_view = ClauseDispatchCompatWords::new(tokens);
    word_view.to_word_refs().join(" ")
}

fn contains_card_type(card_types: &[CardType], target: CardType) -> bool {
    for card_type in card_types {
        if *card_type == target {
            return true;
        }
    }
    false
}

pub(super) fn push_unique_card_type(card_types: &mut Vec<CardType>, card_type: CardType) {
    if !contains_card_type(card_types, card_type) {
        card_types.push(card_type);
    }
}

fn contains_subtype(subtypes: &[Subtype], target: Subtype) -> bool {
    for subtype in subtypes {
        if *subtype == target {
            return true;
        }
    }
    false
}

pub(super) fn push_unique_subtype(subtypes: &mut Vec<Subtype>, subtype: Subtype) {
    if !contains_subtype(subtypes, subtype) {
        subtypes.push(subtype);
    }
}

pub(super) fn parse_controller_or_owner_of_target_subject(
    subject_tokens: &[OwnedLexToken],
) -> Option<(SubjectAst, TargetAst)> {
    let subject_view = ClauseDispatchCompatWords::new(subject_tokens);
    let subject_words = subject_view.to_word_refs();
    let (player, target_start) = match subject_words.as_slice() {
        ["the", "controller", "of", ..] => (PlayerAst::ItsController, 3usize),
        ["controller", "of", ..] => (PlayerAst::ItsController, 2usize),
        ["the", "owner", "of", ..] => (PlayerAst::ItsOwner, 3usize),
        ["owner", "of", ..] => (PlayerAst::ItsOwner, 2usize),
        _ => return None,
    };

    let target_tokens = trim_commas(&subject_tokens[target_start..]);
    if target_tokens.is_empty() {
        return None;
    }

    let target = parse_target_phrase(&target_tokens).ok()?;
    Some((SubjectAst::Player(player), target))
}

fn trim_plural_s(word: &str) -> Option<&str> {
    let bytes = word.as_bytes();
    let last = bytes.last().copied()?;
    if last != b's' && last != b'S' {
        return None;
    }
    word.get(..word.len().saturating_sub(1))
}

pub(super) fn parse_subtype_word_or_plural(word: &str) -> Option<Subtype> {
    parse_subtype_word(word).or_else(|| trim_plural_s(word).and_then(parse_subtype_word))
}

pub(super) fn has_counter_state_pronoun(subject_words: &[&str]) -> bool {
    for start in 0..subject_words.len().saturating_sub(2) {
        if matches!(subject_words[start], "counter" | "counters")
            && subject_words[start + 1] == "on"
            && matches!(subject_words[start + 2], "it" | "them")
        {
            return true;
        }
    }
    false
}

pub(super) fn subject_references_base_power_toughness(subject_words: &[&str]) -> bool {
    find_word_sequence_index(subject_words, &["base", "power", "and", "toughness"]).is_some()
}

pub(super) fn strip_base_power_toughness_subject_tokens<'a>(
    subject_tokens: &'a [OwnedLexToken],
    subject_words: &[&str],
) -> &'a [OwnedLexToken] {
    let Some(base_word_idx) =
        find_word_sequence_index(subject_words, &["base", "power", "and", "toughness"])
    else {
        return subject_tokens;
    };
    let Some(base_token_idx) = token_index_for_word_index(subject_tokens, base_word_idx) else {
        return subject_tokens;
    };

    let mut stripped = &subject_tokens[..base_token_idx];
    while stripped.last().is_some_and(|token| token.is_word("s")) {
        stripped = &stripped[..stripped.len().saturating_sub(1)];
    }
    stripped
}

pub(super) fn parse_become_base_pt_tail<'a>(
    become_words: &'a [&'a str],
) -> Result<Option<(&'a [&'a str], i32, i32)>, CardTextError> {
    let Some(with_idx) = find_word_index(become_words, "with") else {
        return Ok(None);
    };
    let tail = &become_words[with_idx + 1..];
    if tail.len() != 5 || tail[..4] != ["base", "power", "and", "toughness"] {
        return Ok(None);
    }
    let (power, toughness) = parse_pt_modifier(tail[4])?;
    Ok(Some((&become_words[..with_idx], power, toughness)))
}

pub(super) fn parse_become_creature_descriptor_words(
    descriptor_words: &[&str],
) -> Option<(Vec<CardType>, Vec<Subtype>, Option<crate::color::ColorSet>)> {
    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    let mut colors = crate::color::ColorSet::new();
    let mut saw_subtype = false;

    for word in descriptor_words {
        if matches!(*word, "and" | "or") {
            continue;
        }
        if let Some(color) = parse_color(word) {
            colors = colors.union(color);
        } else if let Some(card_type) = parse_card_type(word) {
            push_unique_card_type(&mut card_types, card_type);
        } else if let Some(subtype) = parse_subtype_word_or_plural(word) {
            push_unique_subtype(&mut subtypes, subtype);
            saw_subtype = true;
        } else {
            return None;
        }
    }

    if saw_subtype && !contains_card_type(&card_types, CardType::Creature) {
        card_types.insert(0, CardType::Creature);
    }
    if card_types.is_empty() && !saw_subtype {
        return None;
    }

    Some((
        card_types,
        subtypes,
        if colors.is_empty() {
            None
        } else {
            Some(colors)
        },
    ))
}
