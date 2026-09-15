use super::*;

pub fn parse_base_power_toughness_subject_tokens(
    tokens: &[OwnedLexToken],
) -> Option<BasePowerToughnessSubjectShape<'_>> {
    if let Some((_, target_tokens)) = primitives::parse_prefix(
        tokens,
        primitives::phrase(&["the", "base", "power", "and", "toughness", "of"]),
    ) {
        let target_tokens = crate::lexer::trim_lexed_commas(target_tokens);
        if !target_tokens.is_empty() {
            return Some(BasePowerToughnessSubjectShape { target_tokens });
        }
    }

    let (base_start, _, _) = primitives::find_prefix(tokens, || {
        primitives::phrase(&["base", "power", "and", "toughness"])
    })?;
    let mut target_tokens = tokens.get(..base_start)?;
    while target_tokens.last().is_some_and(|token| token.is_word("s")) {
        target_tokens = &target_tokens[..target_tokens.len().saturating_sub(1)];
    }
    Some(BasePowerToughnessSubjectShape { target_tokens })
}

pub fn parse_filtered_object_animation_tokens(
    tokens: &[OwnedLexToken],
) -> Option<FilteredObjectAnimationShape<'_>> {
    let tokens = crate::lexer::trim_lexed_commas(tokens);
    let word_view = TokenWordView::new(tokens);
    let words = word_view.word_refs();
    if words.is_empty() {
        return None;
    }

    let lose_all = crate::slice_primitives::find_window_by(&words, 4, |window| {
        crate::word_primitives::parse_choice_sequence_complete(
            window,
            &[&["lose", "loses"], &["all"], &["abilities"], &["and"]],
        )
    });
    let subject_word_end = lose_all.unwrap_or(words.len());
    let copula_search_start = lose_all.map_or(0, |start| start + 4);
    let mut parsed = None;
    for copula_word in copula_search_start..words.len() {
        // The lexer may split a contracted pronoun copula ("it's") into two
        // words; treat "it s" as the dependent copula "it's".
        let split_contraction = words[copula_word] == "it"
            && words.get(copula_word + 1) == Some(&"s")
            && copula_word + 2 < words.len();
        if !split_contraction
            && !matches!(
                words[copula_word],
                "is" | "are" | "become" | "becomes" | "its" | "it's" | "it’s"
            )
        {
            continue;
        }
        let body_start = if split_contraction {
            copula_word + 2
        } else {
            copula_word + 1
        };
        let full_body_words = &words[body_start..];
        let Some((body_words, tail)) = split_animation_tail_words(full_body_words) else {
            continue;
        };
        let parsed_body = {
            // "… is a creature with base power and toughness 5/5 in addition
            // to its other types" carries the addition tail after the P/T.
            let (with_body_words, with_addition) = strip_become_addition_tail_words(body_words);
            parse_become_base_pt_words(with_body_words).and_then(|power_toughness| {
                let descriptor =
                    parse_become_creature_descriptor_words(power_toughness.descriptor_words)?;
                Some((
                    power_toughness.power,
                    power_toughness.toughness,
                    descriptor,
                    with_addition,
                ))
            })
        }
            .or_else(|| {
                let body_words =
                    crate::word_primitives::strip_any_prefix(body_words, &[&["a"], &["an"]])
                        .map_or(body_words, |(_, tail)| tail);
                let (descriptor_words, preserve_other_types) =
                    strip_become_addition_tail_words(body_words);
                let leading = parse_become_leading_pt_shape(descriptor_words, &[])?;
                let descriptor = parse_become_creature_descriptor_words(
                    descriptor_words.get(leading.value_word_count..)?,
                )?;
                Some((
                    leading.power,
                    leading.toughness,
                    descriptor,
                    preserve_other_types,
                ))
            });
        let Some((power, toughness, descriptor, preserve_other_types)) = parsed_body else {
            continue;
        };
        if !crate::slice_primitives::contains(
            &descriptor.card_types,
            &crate::types::CardType::Creature,
        ) {
            continue;
        }
        parsed = Some((
            copula_word,
            power,
            toughness,
            descriptor,
            preserve_other_types || tail.still_other_card_type,
            tail,
        ));
        break;
    }
    let (copula_word, power, toughness, descriptor, preserve_other_types, tail) = parsed?;

    let dependent_subject = matches!(words[copula_word], "its" | "it's" | "it’s" | "it");
    let subject_word_end = if lose_all.is_some() {
        subject_word_end
    } else {
        copula_word
    };
    if subject_word_end == 0 && !dependent_subject {
        return None;
    }
    // A targeted subject or a leading one-shot duration ("Until end of turn,
    // target creature becomes ...") is an effect sentence, never a static
    // characteristic statement; the tolerant anthem-subject fallback would
    // otherwise swallow the prefix and mis-scope the animation to every
    // matching object on the battlefield.
    let subject_words = &words[..subject_word_end];
    if crate::word_primitives::contains_word(subject_words, "target")
        || crate::word_primitives::first_is(subject_words, "until")
    {
        return None;
    }
    let subject_token_end = word_view.token_index_after_words(subject_word_end)?;

    Some(FilteredObjectAnimationShape {
        subject_tokens: &tokens[..subject_token_end],
        dependent_subject,
        removes_all_abilities: lose_all.is_some() || tail.loses_all_other_abilities,
        preserve_other_types,
        descriptor,
        power,
        toughness,
        granted_keyword_words: tail.keyword_words,
        still_other_card_type: tail.still_other_card_type,
    })
}

/// Trailing riders after an animation body ("… is a 3/4 Ninja creature and
/// has hexproof", "… creature with indestructible that's still a
/// planeswalker", "… with base power and toughness 0/1 and has
/// indestructible, and it loses all other abilities, card types, and creature
/// types").
#[derive(Debug, Default, Clone)]
struct AnimationTailWords<'a> {
    keyword_words: Vec<&'a str>,
    still_other_card_type: bool,
    loses_all_other_abilities: bool,
}

/// Split the animation body from its trailing riders. Returns `None` when a
/// rider is recognized but malformed, so the caller never silently drops it.
fn split_animation_tail_words<'a, 'b>(
    body: &'b [&'a str],
) -> Option<(&'b [&'a str], AnimationTailWords<'a>)> {
    const HAS_MARKERS: &[&[&str]] = &[
        &["and", "it", "has"],
        &["and", "they", "have"],
        &["and", "has"],
        &["and", "have"],
    ];
    const STILL_MARKERS: &[&[&str]] = &[
        &["that", "s", "still", "a"],
        &["that", "s", "still", "an"],
        &["thats", "still", "a"],
        &["thats", "still", "an"],
        &["that", "is", "still", "a"],
        &["that", "is", "still", "an"],
        &["it", "s", "still", "a"],
        &["it", "s", "still", "an"],
        &["its", "still", "a"],
        &["its", "still", "an"],
    ];
    const LOSES_MARKERS: &[&[&str]] = &[
        &["and", "it", "loses", "all", "other", "abilities"],
        &["and", "loses", "all", "other", "abilities"],
    ];
    const ADDITION_MARKER: &[&str] = &["in", "addition", "to"];

    fn marker_at(words: &[&str], index: usize) -> Option<usize> {
        let rest = &words[index..];
        let is_with = rest.first() == Some(&"with")
            && !matches!(rest.get(1), Some(&"base") | Some(&"power") | Some(&"toughness"))
            && rest.len() > 1;
        if is_with {
            return Some(1);
        }
        for markers in [HAS_MARKERS, STILL_MARKERS, LOSES_MARKERS] {
            if let Some(marker) = markers
                .iter()
                .find(|marker| permission_shapes::prefix_words(rest, marker))
            {
                return Some(marker.len());
            }
        }
        if permission_shapes::prefix_words(rest, ADDITION_MARKER) {
            return Some(0);
        }
        None
    }

    // The tail can only start after the creature noun or after a P/T value.
    let creature_index = body
        .iter()
        .position(|word| matches!(*word, "creature" | "creatures"));
    let Some(creature_index) = creature_index else {
        return Some((body, AnimationTailWords::default()));
    };
    let mut start = None;
    for index in creature_index + 1..body.len() {
        if marker_at(body, index).is_some() {
            // "with base power and toughness N/M" belongs to the body; the tail
            // begins at the first rider after it.
            if body[index] == "in" {
                continue;
            }
            start = Some(index);
            break;
        }
    }
    let Some(start) = start else {
        return Some((body, AnimationTailWords::default()));
    };
    // Keep an addition tail ("in addition to its other types") inside the
    // body so the existing body grammar reads it.
    let body_end = start;
    let mut tail = AnimationTailWords::default();
    let mut index = start;
    while index < body.len() {
        let rest = &body[index..];
        if let Some(marker) = LOSES_MARKERS
            .iter()
            .find(|marker| permission_shapes::prefix_words(rest, marker))
        {
            // "…, card types, and creature types" is part of the same loss.
            let remainder = &rest[marker.len()..];
            if remainder
                .iter()
                .any(|word| !matches!(*word, "card" | "types" | "type" | "and" | "creature"))
            {
                return None;
            }
            tail.loses_all_other_abilities = true;
            break;
        }
        if let Some(marker) = STILL_MARKERS
            .iter()
            .find(|marker| permission_shapes::prefix_words(rest, marker))
        {
            let card_type = rest.get(marker.len())?;
            leaf::parse_leaf_card_type_complete(card_type).ok()?;
            tail.still_other_card_type = true;
            index += marker.len() + 1;
            continue;
        }
        let keyword_start = if rest.first() == Some(&"with") {
            1
        } else if let Some(marker) = HAS_MARKERS
            .iter()
            .find(|marker| permission_shapes::prefix_words(rest, marker))
        {
            marker.len()
        } else {
            return None;
        };
        let mut end = index + keyword_start;
        while end < body.len() {
            if marker_at(body, end).is_some() && body[end] != "in" {
                break;
            }
            end += 1;
        }
        let keywords = &body[index + keyword_start..end];
        if keywords.is_empty() {
            return None;
        }
        tail.keyword_words.extend_from_slice(keywords);
        index = end;
    }
    Some((&body[..body_end], tail))
}
