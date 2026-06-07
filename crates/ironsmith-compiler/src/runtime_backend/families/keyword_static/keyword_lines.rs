#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProtectionChosenTargetShape {
    Player,
    Color,
}

const KEYWORD_PROTECTION_EACH_MANA_VALUE_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::phrase(&["each", "mana", "value", "among"])]);
const KEYWORD_THE_CHOSEN_PLAYER_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::phrase(&["the", "chosen", "player"])]);
const KEYWORD_THE_CHOSEN_COLOR_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::any_phrase(&[
        &["the", "chosen", "color"],
        &["the", "last", "chosen", "color"],
    ])]);
const KEYWORD_PERMANENT_WORDS: &[&str] = &["permanent", "permanents"];
const KEYWORD_COLOR_WORDS: &[&str] = &["color", "colors"];

fn keyword_word_at(words: &[&str], idx: usize, expected: &str) -> bool {
    words.get(idx).copied() == Some(expected)
}

fn keyword_any_word_at(words: &[&str], idx: usize, expected: &[&str]) -> bool {
    words.get(idx).is_some_and(|word| expected.contains(word))
}

fn protection_chosen_target_shape(
    target_tail: LexedClause<'_>,
) -> Option<ProtectionChosenTargetShape> {
    if KEYWORD_THE_CHOSEN_PLAYER_PATTERN
        .match_prefix(target_tail)
        .is_some()
    {
        return Some(ProtectionChosenTargetShape::Player);
    }
    if KEYWORD_THE_CHOSEN_COLOR_PATTERN
        .match_prefix(target_tail)
        .is_some()
    {
        return Some(ProtectionChosenTargetShape::Color);
    }
    None
}

pub(crate) fn parse_ability_line(tokens: &[OwnedLexToken]) -> Option<Vec<KeywordAction>> {
    if let Some(actions) = parse_flashback_keyword_line(tokens) {
        return Some(actions);
    }
    let words = crate::runtime_backend::lexer::TokenWordView::new(tokens).word_refs();
    if let Some(action) =
        super::activation_and_restrictions::parse_dynamic_soulshift_keyword_action(&words)
    {
        return Some(vec![action]);
    }
    if let Some(action @ KeywordAction::CumulativeUpkeep { .. }) = parse_ability_phrase(tokens) {
        return Some(vec![action]);
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
    let clause = LexedClause::new(tokens);
    let words_view = crate::runtime_backend::lexer::TokenWordView::new(tokens);
    let words = words_view.word_refs();
    let first_word_idx = if keyword_word_at(&words, 0, "and") {
        1
    } else {
        0
    };
    if words.len().saturating_sub(first_word_idx) < 3 {
        return None;
    }
    if !keyword_word_at(&words, first_word_idx, "protection")
        || !keyword_word_at(&words, first_word_idx + 1, "from")
    {
        return None;
    }

    let mut actions = Vec::new();
    let parse_from_target = |words: &[&str], idx: usize| -> Option<KeywordAction> {
        let value = *words.get(idx + 1)?;
        let target_tail = clause.from_word(idx + 1)?;
        if KEYWORD_PROTECTION_EACH_MANA_VALUE_PATTERN
            .match_prefix(target_tail)
            .is_some()
        {
            let filter_tokens = trim_commas(clause.from_word(idx + 5)?.tokens());
            let filter = parse_object_filter_lexed(&filter_tokens, false).ok()?;
            return Some(KeywordAction::ProtectionFromEachManaValueAmong(filter));
        }
        if value == "spells" || value == "spell" {
            return Some(KeywordAction::ProtectionFromFilter(ObjectFilter::spell()));
        }
        if matches!(value, "permanent" | "permanents")
            && words.get(idx + 2..idx + 7) == Some(&["that", "were", "cast", "this", "turn"][..])
        {
            let mut filter = ObjectFilter::permanent();
            filter.cast_this_turn = true;
            return Some(KeywordAction::ProtectionFromFilter(filter));
        }
        if value == "mana" && words.get(idx + 2).copied() == Some("value") {
            let comparison_tail = words.get(idx + 3..)?;
            let (comparison, consumed) =
                parse_filter_comparison_tokens("mana value", comparison_tail, words).ok()??;
            if consumed == comparison_tail.len() {
                let mut filter = ObjectFilter::default();
                filter.mana_value = Some(comparison);
                return Some(KeywordAction::ProtectionFromFilter(filter));
            }
        }
        if KEYWORD_PERMANENT_WORDS.contains(&value) && keyword_word_at(words, idx + 2, "with") {
            let counter_words = &words[idx + 3..];
            if let Some((with_counter, consumed)) =
                parse_filter_counter_constraint_words(counter_words)
                && consumed == counter_words.len()
            {
                let mut filter = ObjectFilter::permanent();
                filter.with_counter = Some(with_counter);
                return Some(KeywordAction::ProtectionFromFilter(filter));
            }
        }
        match value {
            "the"
                if protection_chosen_target_shape(target_tail)
                    == Some(ProtectionChosenTargetShape::Player) =>
            {
                Some(KeywordAction::ProtectionFromChosenPlayer)
            }
            "the"
                if protection_chosen_target_shape(target_tail)
                    == Some(ProtectionChosenTargetShape::Color) =>
            {
                Some(KeywordAction::ProtectionFromChosenColor)
            }
            "colorless" => Some(KeywordAction::ProtectionFromColorless),
            "everything" => Some(KeywordAction::ProtectionFromEverything),
            "all" if keyword_any_word_at(words, idx + 2, KEYWORD_COLOR_WORDS) => {
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
