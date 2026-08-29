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
        if !matches!(
            words[copula_word],
            "is" | "are" | "become" | "becomes" | "its" | "it's" | "it’s"
        ) {
            continue;
        }
        let body_words = &words[copula_word + 1..];
        let parsed_body = parse_become_base_pt_words(body_words)
            .and_then(|power_toughness| {
                let descriptor =
                    parse_become_creature_descriptor_words(power_toughness.descriptor_words)?;
                Some((
                    power_toughness.power,
                    power_toughness.toughness,
                    descriptor,
                    false,
                ))
            })
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
            preserve_other_types,
        ));
        break;
    }
    let (copula_word, power, toughness, descriptor, preserve_other_types) = parsed?;

    let dependent_subject = matches!(words[copula_word], "its" | "it's" | "it’s");
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
        removes_all_abilities: lose_all.is_some(),
        preserve_other_types,
        descriptor,
        power,
        toughness,
    })
}
