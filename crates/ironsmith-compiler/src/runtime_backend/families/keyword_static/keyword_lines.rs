fn parser_text_words(tokens: &[OwnedLexToken]) -> Vec<&str> {
    tokens
        .iter()
        .filter_map(|token| match token.kind {
            TokenKind::Word | TokenKind::Number | TokenKind::Tilde => Some(token.parser_text()),
            _ => None,
        })
        .collect()
}

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
    let words = parser_text_words(tokens);
    let first_word_idx = if words.first().copied() == Some("and") {
        1
    } else {
        0
    };
    if words.len().saturating_sub(first_word_idx) < 3 {
        return None;
    }
    if words.get(first_word_idx).copied() != Some("protection")
        || words.get(first_word_idx + 1).copied() != Some("from")
    {
        return None;
    }

    let mut actions = Vec::new();
    let parse_from_target = |words: &[&str], idx: usize| -> Option<KeywordAction> {
        let value = *words.get(idx + 1)?;
        match value {
            "the"
                if words.get(idx + 2).copied() == Some("chosen")
                    && words.get(idx + 3).copied() == Some("player") =>
            {
                Some(KeywordAction::ProtectionFromChosenPlayer)
            }
            "colorless" => Some(KeywordAction::ProtectionFromColorless),
            "everything" => Some(KeywordAction::ProtectionFromEverything),
            "all" if matches!(words.get(idx + 2).copied(), Some("color") | Some("colors")) => {
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
