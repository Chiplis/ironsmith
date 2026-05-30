pub(crate) fn parse_ability_line(tokens: &[OwnedLexToken]) -> Option<Vec<KeywordAction>> {
    if let Some(actions) = parse_flashback_keyword_line(tokens) {
        return Some(actions);
    }

    let segments = split_lexed_slices_on_commas_or_semicolons(tokens);
    let mut actions = Vec::new();

    for segment in segments {
        if segment.is_empty() {
            continue;
        }

        if let Some(protection_actions) = parse_protection_chain(segment) {
            actions.extend(protection_actions);
            continue;
        }

        // Try the segment as-is first, then split on "and" for compound keywords
        if let Some(action) = parse_ability_phrase(segment) {
            actions.push(action);
        } else {
            // Split on "and" to handle "menace and deathtouch", "trample and haste", etc.
            let and_parts = split_lexed_slices_on_and(segment);
            if and_parts.len() > 1 {
                let mut all_ok = true;
                for part in and_parts {
                    if part.is_empty() {
                        continue;
                    }
                    if let Some(action) = parse_ability_phrase(part) {
                        actions.push(action);
                    } else {
                        all_ok = false;
                        break;
                    }
                }
                if !all_ok {
                    return None;
                }
            } else {
                return None;
            }
        }
    }

    if actions.is_empty() {
        None
    } else {
        Some(actions)
    }
}

pub(crate) fn reject_unimplemented_keyword_actions(
    _actions: &[KeywordAction],
    _clause: &str,
) -> Result<(), CardTextError> {
    Ok(())
}

pub(crate) fn parse_protection_chain(tokens: &[OwnedLexToken]) -> Option<Vec<KeywordAction>> {
    let words_view = crate::runtime_backend::lexer::TokenWordView::new(tokens);
    let words = words_view.word_refs();
    let first_word_idx = if word_slice_at_is(&words, 0, "and") {
        1
    } else {
        0
    };
    if words.len().saturating_sub(first_word_idx) < 3 {
        return None;
    }
    if !word_slice_at_is(&words, first_word_idx, "protection")
        || !word_slice_at_is(&words, first_word_idx + 1, "from")
    {
        return None;
    }

    let mut actions = Vec::new();
    let parse_from_target = |words: &[&str], idx: usize| -> Option<KeywordAction> {
        let value = *words.get(idx + 1)?;
        if value == "each"
            && word_slice_at_is(words, idx + 2, "mana")
            && word_slice_at_is(words, idx + 3, "value")
            && word_slice_at_is(words, idx + 4, "among")
        {
            let filter_tokens = trim_commas(
                LexedClause::new(tokens)
                    .from_word(idx + 5)?
                    .tokens(),
            );
            let filter = parse_object_filter_lexed(&filter_tokens, false).ok()?;
            return Some(KeywordAction::ProtectionFromEachManaValueAmong(filter));
        }
        if matches!(value, "permanent" | "permanents")
            && word_slice_at_is(words, idx + 2, "with")
        {
            let counter_words = &words[idx + 3..];
            if let Some((with_counter, consumed)) = parse_filter_counter_constraint_words(counter_words)
                && consumed == counter_words.len()
            {
                let mut filter = ObjectFilter::permanent();
                filter.with_counter = Some(with_counter);
                return Some(KeywordAction::ProtectionFromFilter(filter));
            }
        }
        match value {
            "the"
                if word_slice_at_is(words, idx + 2, "chosen")
                    && word_slice_at_is(words, idx + 3, "player") =>
            {
                Some(KeywordAction::ProtectionFromChosenPlayer)
            }
            "the"
                if word_slice_at_is(words, idx + 2, "chosen")
                    && word_slice_at_is(words, idx + 3, "color") =>
            {
                Some(KeywordAction::ProtectionFromChosenColor)
            }
            "the"
                if word_slice_at_is(words, idx + 2, "last")
                    && word_slice_at_is(words, idx + 3, "chosen")
                    && word_slice_at_is(words, idx + 4, "color") =>
            {
                Some(KeywordAction::ProtectionFromChosenColor)
            }
            "colorless" => Some(KeywordAction::ProtectionFromColorless),
            "everything" => Some(KeywordAction::ProtectionFromEverything),
            "all" if word_slice_at_is_any(words, idx + 2, &["color", "colors"]) => {
                Some(KeywordAction::ProtectionFromAllColors)
            }
            _ => parse_color(value)
                .map(KeywordAction::ProtectionFrom)
                .or_else(|| parse_card_type(value).map(KeywordAction::ProtectionFromCardType))
                .or_else(|| {
                    parse_subtype_flexible(value).map(KeywordAction::ProtectionFromSubtype)
                }),
        }
    };

    let has_action = |actions: &[KeywordAction], expected: &KeywordAction| -> bool {
        let mut idx = 0usize;
        while idx < actions.len() {
            if &actions[idx] == expected {
                return true;
            }
            idx += 1;
        }
        false
    };

    let mut from_count = 0usize;
    let mut parsed_count = 0usize;
    for idx in first_word_idx..words.len().saturating_sub(1) {
        if words.get(idx).copied() != Some("from") {
            continue;
        }
        from_count += 1;
        if let Some(action) = parse_from_target(&words, idx) {
            parsed_count += 1;
            if !has_action(&actions, &action) {
                actions.push(action);
            }
        }
    }

    if actions.is_empty() || parsed_count < from_count {
        None
    } else {
        Some(actions)
    }
}

pub(crate) fn keyword_action_to_static_ability(action: KeywordAction) -> Option<StaticAbility> {
    static_ability_for_keyword_action(action)
}
